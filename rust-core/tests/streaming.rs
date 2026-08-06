//! Integration tests for the streaming pipeline: symphonia decoding of a
//! local file, and the HTTP Range stream (download + seek) against a tiny
//! in-process HTTP server.

use rhythm_core::audio::decoder::AudioDecoder;
use rhythm_core::audio::http_stream::HttpStream;
use symphonia::core::io::MediaSource;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Minimal HTTP server that serves a fixed body with full Range support.
/// Speaks just enough HTTP/1.1 for reqwest: HEAD-less GET, Range headers,
/// 200/206 responses, Content-Range/Content-Length.
struct RangeServer {
    addr: String,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RangeServer {
    fn start(body: Vec<u8>) -> Self {
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
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://{}/test", self.addr)
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

/// Build a minimal valid stereo 16-bit WAV file (1 second, 44100 Hz).
fn make_wav_bytes() -> Vec<u8> {
    let sample_rate = 44100u32;
    let channels = 2u16;
    let bits = 16u16;
    let duration_samples = sample_rate as usize; // 1 second
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

#[test]
fn test_decoder_decodes_local_wav() {
    let wav = make_wav_bytes();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tone.wav");
    std::fs::write(&path, &wav).unwrap();

    let mut decoder = AudioDecoder::open_file(&path).unwrap();
    assert_eq!(decoder.sample_rate(), 44100);
    assert_eq!(decoder.channels(), 2);
    assert!((decoder.duration() - 1.0).abs() < 0.05);

    let mut total_frames = 0usize;
    let mut saw_samples = false;
    while let Some(pcm) = decoder.next_packet().unwrap() {
        assert_eq!(pcm.len() % 2, 0);
        total_frames += pcm.len() / 2;
        if pcm.iter().any(|&s| s > 0.1) {
            saw_samples = true;
        }
    }
    // All 44100 frames decoded, and the sine wave actually produced audio.
    assert_eq!(total_frames, 44100);
    assert!(saw_samples);
}

#[test]
fn test_decoder_seek_local_wav() {
    let wav = make_wav_bytes();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tone.wav");
    std::fs::write(&path, &wav).unwrap();

    let mut decoder = AudioDecoder::open_file(&path).unwrap();
    decoder.seek(0.5).unwrap();
    let first = decoder.next_packet().unwrap().unwrap();
    // After seeking to 0.5s, position should be near 0.5s, not 0.
    assert!(decoder.position() > 0.4, "position was {}", decoder.position());
    assert!(!first.is_empty());
}

#[test]
fn test_http_stream_downloads_all_bytes() {
    let body = (0..1024u32).flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>();
    let server = RangeServer::start(body.clone());
    let mut stream = HttpStream::open(&server.url()).unwrap();
    stream.wait_initial_buffered().unwrap();

    let mut downloaded = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).unwrap();
        if n == 0 {
            break;
        }
        downloaded.extend_from_slice(&chunk[..n]);
    }
    assert_eq!(downloaded, body);
    assert_eq!(stream.byte_len(), Some(4096));
    assert!(stream.is_seekable());
}

#[test]
fn test_http_stream_seek_backwards_issues_range() {
    let body = (0..4096u32).flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>();
    let server = RangeServer::start(body.clone());

    let mut stream = HttpStream::open(&server.url()).unwrap();
    // Read the first 2048 bytes.
    let mut buf = [0u8; 2048];
    stream.read(&mut buf).unwrap();
    assert_eq!(buf[..4], 0u32.to_le_bytes());

    // Seek back to byte 1024 (= u32 index 256) and read from there. Reads may
    // be short (prefetch may not have delivered everything yet), so loop.
    stream.seek(SeekFrom::Start(1024)).unwrap();
    let mut got = Vec::new();
    let mut buf2 = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf2).unwrap();
        if n == 0 {
            break;
        }
        got.extend_from_slice(&buf2[..n]);
    }
    let mut expected = Vec::new();
    for i in 256..4096u32 {
        expected.extend_from_slice(&i.to_le_bytes());
    }
    assert_eq!(got, expected);
    assert_eq!(stream.byte_len(), Some(16384));
}

#[test]
fn test_http_stream_forward_seek_within_buffer() {
    let body = (0..4096u32).flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>();
    let server = RangeServer::start(body.clone());

    let mut stream = HttpStream::open(&server.url()).unwrap();
    stream.wait_initial_buffered().unwrap();

    // Seek forward to byte 2048 (= u32 index 512) — inside the already
    // buffered window, so no network round trip is needed. Reads may be
    // short; loop until EOF.
    stream.seek(SeekFrom::Start(2048)).unwrap();
    let mut got = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        got.extend_from_slice(&buf[..n]);
    }
    let mut expected = Vec::new();
    for i in 512..4096u32 {
        expected.extend_from_slice(&i.to_le_bytes());
    }
    assert_eq!(got, expected);
}

#[test]
fn test_http_stream_blocks_until_initial_buffer() {
    let body = (0..1024u32).flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>();
    let server = RangeServer::start(body);
    let mut stream = HttpStream::open(&server.url()).unwrap();

    // Reading immediately (before wait_initial_buffered) should still return
    // data once the prefetch thread fills the buffer; it must never return
    // partial garbage.
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap();
    assert!(n > 0);
    assert_eq!(buf[..4], 0u32.to_le_bytes());
    let _ = &mut stream;
    thread::sleep(Duration::from_millis(100));
}


/// `SeekFrom::Current` is resolved against `read_pos`, which used to be
/// updated only by `read()`. A relative seek issued straight after an absolute
/// one therefore computed its offset from a stale position — the pattern
/// symphonia's MP4 probe uses on every open (#23).
#[test]
fn test_http_stream_relative_seek_after_absolute_seek() {
    let body = (0..4096u32).flat_map(|i| i.to_le_bytes()).collect::<Vec<_>>();
    let server = RangeServer::start(body);

    let mut stream = HttpStream::open(&server.url()).unwrap();
    stream.wait_initial_buffered().unwrap();

    // Absolute seek, then a relative no-op before any read: the reported
    // position must not drift.
    stream.seek(SeekFrom::Start(2048)).unwrap();
    let here = stream.seek(SeekFrom::Current(0)).unwrap();
    assert_eq!(here, 2048, "relative seek resolved against a stale read_pos");

    // And a relative step must land where it claims.
    let next = stream.seek(SeekFrom::Current(1024)).unwrap();
    assert_eq!(next, 3072);

    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(
        u32::from_le_bytes(buf),
        768,
        "byte 3072 is u32 index 768; reading elsewhere means the seek lied"
    );
}
