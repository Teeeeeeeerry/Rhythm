//! Integration tests for the streaming pipeline: symphonia decoding of a
//! local file, and the HTTP Range stream (download + seek) against a tiny
//! in-process HTTP server (shared `common` module).

mod common;

use common::{make_wav_bytes, RangeServer};
use rhythm_core::audio::decoder::AudioDecoder;
use rhythm_core::audio::http_stream::HttpStream;
use symphonia::core::io::MediaSource;
use std::io::{Read, Seek, SeekFrom};
use std::thread;
use std::time::Duration;

#[test]
fn test_decoder_decodes_local_wav() {
    let wav = make_wav_bytes(1.0);
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
    let wav = make_wav_bytes(1.0);
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
    stream.read_exact(&mut buf).unwrap();
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
    // position must not drift. `Current(0)` is deliberately tested as the
    // relative-seek no-op (#143, clippy::seek_from_current).
    stream.seek(SeekFrom::Start(2048)).unwrap();
    #[allow(clippy::seek_from_current)]
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
