//! Shared test infrastructure: an in-process HTTP Range server, a WAV
//! builder, and tag-writing fixtures. The Range server is used by
//! `streaming.rs` (decoder / HttpStream behavior) and `audio_engine.rs`
//! (AudioEngine state machine behavior); the WAV/tag fixtures by
//! `library_behavior.rs` and `metadata_behavior.rs`.
//!
//! Each test binary compiles this module separately and uses only a subset
//! of the fixtures, so dead-code warnings are expected (#143).
#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Minimal HTTP server that serves a fixed body with full Range support.
/// Speaks just enough HTTP/1.1 for reqwest: HEAD-less GET, Range headers,
/// 200/206 responses, Content-Range/Content-Length.
pub struct RangeServer {
    addr: String,
    path: String,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RangeServer {
    // Not every test target uses both helpers (streaming.rs uses this one,
    // audio_engine.rs uses start_with_path), so keep the dead-code lint quiet.
    #[allow(dead_code)]
    pub fn start(body: Vec<u8>) -> Self {
        Self::start_with_path(body, "/test")
    }

    /// Serve `body` at `path` (use e.g. "/tone.wav" so URL-extension probe
    /// hints resolve).
    pub fn start_with_path(body: Vec<u8>, path: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let server_body = body;
        let handle = thread::spawn(move || {
            // Non-blocking accept loop so the thread can observe the stop
            // flag instead of blocking on accept forever.
            listener.set_nonblocking(true).unwrap();
            loop {
                if stop2.load(Ordering::SeqCst) {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // The listener is non-blocking so the accept loop can
                        // poll the stop flag. macOS accepts inherit that flag:
                        // without this reset, the handler's blocking reads and
                        // its large-response writes fail with WouldBlock and
                        // get silently truncated. Bodies over the socket
                        // buffer size (e.g. the audio_engine WAV fixtures)
                        // reproduce it every time.
                        stream.set_nonblocking(false).unwrap();
                        let body = server_body.clone();
                        thread::spawn(move || handle_connection(&mut stream, &body));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        RangeServer {
            addr,
            path: path.to_string(),
            stop,
            handle: Some(handle),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}{}", self.addr, self.path)
    }
}

impl Drop for RangeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn handle_connection(stream: &mut TcpStream, body: &[u8]) {
    // Read the request head.
    let mut buf = [0u8; 8192];
    let mut head = Vec::new();
    loop {
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(_) => return,
        };
        if n == 0 {
            return;
        }
        head.extend_from_slice(&buf[..n]);
        if head.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let head = String::from_utf8_lossy(&head);
    let mut range: Option<(u64, u64)> = None;
    for line in head.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(r) = lower.strip_prefix("range: bytes=") {
            let r = r.trim();
            if let Some(end) = r.find('-') {
                let start: u64 = r[..end].parse().unwrap_or(0);
                let end: u64 = r[end + 1..].parse().unwrap_or(body.len() as u64 - 1);
                range = Some((start, end));
            }
        }
    }

    let total = body.len() as u64;
    let (status, content_range, start, end) = match range {
        Some((start, _end)) if start >= total => {
            // An open-ended range starting past the body (what a seek to the
            // very end emits) must get 416, not an index panic in the
            // detached handler thread.
            let response = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{total}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(response.as_bytes());
            return;
        }
        Some((start, end)) => {
            let start = start.min(total);
            let end = end.min(total.saturating_sub(1)).max(start);
            (
                "206 Partial Content",
                format!("bytes {start}-{end}/{total}"),
                start,
                end,
            )
        }
        None => ("200 OK", String::new(), 0, total.saturating_sub(1)),
    };

    let chunk = &body[start as usize..=end as usize];
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\nAccept-Ranges: bytes\r\n{}{}\r\n",
        chunk.len(),
        if content_range.is_empty() {
            String::new()
        } else {
            format!("Content-Range: {content_range}\r\n")
        },
        ""
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(chunk);
    let _ = stream.flush();
}

/// Build a minimal valid stereo 16-bit WAV file of `seconds` seconds at
/// 44100 Hz (a 440 Hz sine wave on the left channel, silence on the right).
pub fn make_wav_bytes(seconds: f64) -> Vec<u8> {
    let sample_rate = 44100u32;
    let channels = 2u16;
    let bits = 16u16;
    let duration_samples = (sample_rate as f64 * seconds) as usize;
    let data_len = duration_samples * channels as usize * 2;
    let mut wav = Vec::with_capacity(44 + data_len);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
    wav.extend_from_slice(&(channels * bits / 8).to_le_bytes());
    wav.extend_from_slice(&bits.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_len as u32).to_le_bytes());

    // A 440 Hz sine wave on the left channel, silence on the right.
    for i in 0..duration_samples {
        let v = (440.0 * 2.0 * std::f64::consts::PI * i as f64 / sample_rate as f64).sin();
        let sample = (v * 32767.0) as i16;
        wav.extend_from_slice(&sample.to_le_bytes());
        wav.extend_from_slice(&0i16.to_le_bytes());
    }
    wav
}

// ─── Metadata fixtures ───────────────────────────────────────────────

/// Write `seconds` of WAV silence-tone to `path` (no tags).
pub fn write_wav(path: &std::path::Path, seconds: f64) {
    std::fs::write(path, make_wav_bytes(seconds)).unwrap();
}

/// Write a WAV file and attach an ID3v2 tag configured by `configure`
/// (title/artist/album/track/…, or a picture via `Tag::push_picture`).
pub fn write_tagged_wav(
    path: &std::path::Path,
    seconds: f64,
    configure: impl FnOnce(&mut lofty::tag::Tag),
) {
    use lofty::prelude::*;
    write_wav(path, seconds);
    let mut tag = lofty::tag::Tag::new(lofty::tag::TagType::Id3v2);
    configure(&mut tag);
    tag.save_to_path(path, lofty::config::WriteOptions::default())
        .unwrap();
}

// ─── Track fixtures ──────────────────────────────────────────────────

/// A local track with sensible defaults (the same shape as
/// `library_integration.rs::dummy_local_track`, centralized so the
/// behavior suites don't repeat the 19-field literal).
pub fn test_local_track(
    path: &str,
    title: &str,
    artist: Option<&str>,
    duration: f64,
) -> rhythm_core::TrackInfo {
    use rhythm_core::{SourceType, TrackInfo};
    TrackInfo {
        id: None,
        file_path: Some(path.to_string()),
        source_type: SourceType::Local,
        source_url: None,
        title: title.to_string(),
        artist: artist.map(String::from),
        album: Some("Test Album".to_string()),
        album_artist: None,
        track_number: Some(1),
        disc_number: Some(1),
        genre: None,
        year: Some(2024),
        duration,
        format: Some("mp3".to_string()),
        bitrate: Some(320),
        sample_rate: Some(44100),
        channels: Some(2),
        file_size: Some(5_000_000),
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: None,
        is_available: true,
    }
}

/// A URL track with sensible defaults.
pub fn test_url_track(
    url: &str,
    title: &str,
    artist: Option<&str>,
    source_type: rhythm_core::SourceType,
    duration: f64,
) -> rhythm_core::TrackInfo {
    use rhythm_core::TrackInfo;
    TrackInfo {
        id: None,
        file_path: None,
        source_type,
        source_url: Some(url.to_string()),
        title: title.to_string(),
        artist: artist.map(String::from),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        genre: None,
        year: None,
        duration,
        format: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_size: None,
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: None,
        is_available: true,
    }
}

// ─── Environment guard ───────────────────────────────────────────────

/// Set an environment variable for the test's duration and restore the
/// previous value (or unset it) on drop — panic-safe where manual
/// save/restore pairs would leak on early unwinds.
pub struct EnvGuard {
    key: &'static str,
    old: Option<std::ffi::OsString>,
}

impl EnvGuard {
    pub fn set(key: &'static str, value: impl AsRef<std::path::Path>) -> Self {
        let old = std::env::var_os(key);
        std::env::set_var(key, value.as_ref());
        EnvGuard { key, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}
