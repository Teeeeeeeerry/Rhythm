use crate::{RhythmError, RhythmResult};
use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use symphonia::core::io::MediaSource;

/// Prefetch watermarks (bytes). The downloader keeps the shared buffer topped
/// up to `HIGH_WATER`, and playback starts once `INITIAL_WATER` bytes have
/// arrived, so short network stalls are absorbed by the buffer.
const CHUNK_SIZE: usize = 64 * 1024;
const HIGH_WATER: usize = 1024 * 1024;
const INITIAL_WATER: usize = 256 * 1024;

/// Shared state between the download (prefetch) thread and the reader.
struct HttpBuffer {
    /// Buffered bytes. The byte at `data[0]` is at absolute position `pos`.
    data: VecDeque<u8>,
    /// Absolute byte position of the first buffered byte (bytes consumed so far).
    pos: u64,
    /// Total length in bytes, if the server reported one.
    total_len: Option<u64>,
    /// Set once the current response stream is exhausted.
    eof: bool,
    /// Fatal download error surfaced to the reader.
    error: Option<String>,
    /// Whether the server honors Range requests (responds with 206).
    range_supported: bool,
    /// Bumped on every (re)start of a request; lets the downloader drop bytes
    /// from a response that has been invalidated by a seek.
    generation: u64,
}

/// HTTP byte stream with prefetch buffering and Range-based seeking.
///
/// Implements `Read + Seek + MediaSource` so it can be fed directly into the
/// symphonia decoder. A background thread downloads ahead of the reader into a
/// shared buffer; `seek` either rewinds within the buffered window or issues a
/// new `Range: bytes=N-` request (servers without Range support can only seek
/// forward within the buffered window).
pub struct HttpStream {
    inner: Arc<HttpStreamInner>,
    read_pos: u64,
}

struct HttpStreamInner {
    client: reqwest::blocking::Client,
    url: String,
    buffer: Mutex<HttpBuffer>,
    /// Signals buffer changes (new data, seek, eof, error, stop).
    /// Always paired with the `buffer` mutex.
    cond: Condvar,
    /// Signals a new response in the `downloading` slot.
    /// Always paired with the `downloading` mutex.
    resp_cond: Condvar,
    stop: AtomicBool,
    handle: Mutex<Option<thread::JoinHandle<()>>>,
    /// Slot where the latest `Response` is handed to the downloader thread.
    /// Carries the generation of the request that produced it.
    downloading: Mutex<Option<(u64, reqwest::blocking::Response)>>,
}

impl HttpStream {
    /// Start streaming `url`. The initial request (headers + Range support
    /// detection) is issued synchronously so errors surface immediately.
    pub fn open(url: &str) -> RhythmResult<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("Rhythm/0.1")
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RhythmError::Network(format!("Failed to build HTTP client: {e}")))?;

        let inner = Arc::new(HttpStreamInner {
            client,
            url: url.to_string(),
            buffer: Mutex::new(HttpBuffer {
                data: VecDeque::with_capacity(HIGH_WATER),
                pos: 0,
                total_len: None,
                eof: false,
                error: None,
                range_supported: false,
                generation: 0,
            }),
            cond: Condvar::new(),
            resp_cond: Condvar::new(),
            stop: AtomicBool::new(false),
            handle: Mutex::new(None),
            downloading: Mutex::new(None),
        });

        let stream = HttpStream {
            inner: inner.clone(),
            read_pos: 0,
        };

        let handle = thread::spawn(move || downloader(inner));
        *stream.inner.handle.lock().unwrap() = Some(handle);

        // Issue the first request now so connection + Range negotiation happen
        // before any read.
        stream.inner.request_range(0)?;

        Ok(stream)
    }

    /// Block until at least `INITIAL_WATER` bytes are buffered (or the stream
    /// ends or errors). Called before playback starts.
    pub fn wait_initial_buffered(&self) -> RhythmResult<()> {
        let mut buffer = self.inner.buffer.lock().unwrap();
        while buffer.data.len() < INITIAL_WATER && !buffer.eof && buffer.error.is_none() {
            buffer = self.inner.cond.wait(buffer).unwrap();
        }
        if let Some(err) = buffer.error.clone() {
            return Err(RhythmError::Network(err));
        }
        Ok(())
    }
}

impl HttpStreamInner {
    /// (Re)start a GET request with a `Range: bytes=offset-` header and reset
    /// the shared buffer. Blocks until response headers arrive, so seek
    /// support and content length are known before any body bytes are read.
    fn request_range(&self, offset: u64) -> RhythmResult<()> {
        // 1. Reset shared state (and bump the generation so the downloader
        //    drops any in-flight bytes from the previous response).
        {
            let mut buffer = self.buffer.lock().unwrap();
            buffer.data.clear();
            buffer.error = None;
            buffer.eof = false;
            buffer.pos = offset;
            buffer.generation += 1;
        }

        // 2. Issue the request without holding any lock.
        let resp = self
            .client
            .get(&self.url)
            .header(reqwest::header::RANGE, format!("bytes={offset}-"))
            .send()
            .map_err(|e| RhythmError::Network(format!("GET {} failed: {e}", self.url)))?;
        if !resp.status().is_success() {
            return Err(RhythmError::Network(format!(
                "GET {} failed: HTTP {}",
                self.url,
                resp.status()
            )));
        }

        // 3. Record response metadata.
        let mut buffer = self.buffer.lock().unwrap();
        let gen = buffer.generation;
        buffer.range_supported = resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;

        if let Some(range) = resp.headers().get(reqwest::header::CONTENT_RANGE) {
            // Content-Range: bytes 0-999/10000
            if let Ok(s) = range.to_str() {
                if let Some(total) = s.rsplit('/').next() {
                    if let Ok(len) = total.parse::<u64>() {
                        buffer.total_len = Some(len);
                    }
                }
            }
        } else if let Some(len) = resp.headers().get(reqwest::header::CONTENT_LENGTH) {
            if let Ok(v) = len.to_str() {
                if let Ok(len) = v.parse::<u64>() {
                    buffer.total_len = Some(offset + len);
                }
            }
        }
        drop(buffer);

        // 4. Hand the response to the downloader thread.
        *self.downloading.lock().unwrap() = Some((gen, resp));
        self.resp_cond.notify_all();
        Ok(())
    }
}

/// Background download loop: pulls bytes from the current response into the
/// shared buffer until EOF, error, or the response is invalidated by a seek.
fn downloader(inner: Arc<HttpStreamInner>) {
    loop {
        if inner.stop.load(Ordering::SeqCst) {
            return;
        }

        // Take the latest response, waiting if none is pending.
        let (gen, mut resp) = {
            let mut downloading = inner.downloading.lock().unwrap();
            if downloading.is_none() {
                downloading = inner.resp_cond.wait(downloading).unwrap();
                continue;
            }
            downloading.take().unwrap()
        };

        let mut chunk = vec![0u8; CHUNK_SIZE];
        loop {
            let read = match resp.read(&mut chunk) {
                Ok(0) => {
                    // EOF of this response. Only flag the stream finished if
                    // it wasn't invalidated by a seek in the meantime.
                    let mut buffer = inner.buffer.lock().unwrap();
                    if buffer.generation == gen {
                        buffer.eof = true;
                        inner.cond.notify_all();
                    }
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    let mut buffer = inner.buffer.lock().unwrap();
                    if buffer.generation == gen {
                        buffer.error = Some(format!("Download error: {e}"));
                        buffer.eof = true;
                        inner.cond.notify_all();
                    }
                    break;
                }
            };

            {
                let mut buffer = inner.buffer.lock().unwrap();
                // Response invalidated by a seek: discard this chunk and pick
                // up the new response in the outer loop.
                if buffer.generation != gen {
                    break;
                }
                buffer.data.extend(&chunk[..read]);
                inner.cond.notify_all();
            }

            // Throttle: don't run ahead of the reader by more than HIGH_WATER.
            let mut buffer = inner.buffer.lock().unwrap();
            while buffer.data.len() > HIGH_WATER
                && buffer.generation == gen
                && !inner.stop.load(Ordering::SeqCst)
            {
                buffer = inner
                    .cond
                    .wait_timeout(buffer, Duration::from_millis(50))
                    .unwrap()
                    .0;
            }
            if inner.stop.load(Ordering::SeqCst) || buffer.generation != gen {
                break;
            }
        }
    }
}

impl Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut buffer = self.inner.buffer.lock().unwrap();

        // Wait for data, EOF, or error.
        while buffer.data.is_empty() && !buffer.eof && buffer.error.is_none() {
            buffer = self.inner.cond.wait(buffer).unwrap();
        }

        if let Some(err) = buffer.error.clone() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
        }
        if buffer.data.is_empty() {
            return Ok(0); // EOF
        }

        let n = buf.len().min(buffer.data.len());
        for (slot, byte) in buf.iter_mut().take(n).zip(buffer.data.drain(..n)) {
            *slot = byte;
        }
        buffer.pos += n as u64;
        self.read_pos = buffer.pos;
        self.inner.cond.notify_all();
        Ok(n)
    }
}

impl Seek for HttpStream {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::Current(delta) => self.read_pos as i64 + delta,
            SeekFrom::End(delta) => {
                let len = self.inner.buffer.lock().unwrap().total_len;
                match len {
                    Some(len) => len as i64 + delta,
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Unsupported,
                            "SeekFrom::End requires a known content length",
                        ));
                    }
                }
            }
        };
        if target < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Seek position is negative",
            ));
        }
        let target = target as u64;
        self.seek_to(target)?;
        Ok(target)
    }
}

impl HttpStream {
    fn seek_to(&self, target: u64) -> std::io::Result<()> {
        let mut buffer = self.inner.buffer.lock().unwrap();

        // Fast path: target lies inside the buffered window — just drop the
        // leading bytes, no network round trip.
        let window_end = buffer.pos + buffer.data.len() as u64;
        if target >= buffer.pos && target <= window_end {
            let skip = (target - buffer.pos) as usize;
            // Consume the drain iterator fully; dropping it unconsumed does
            // not reliably remove the elements.
            for _ in buffer.data.drain(..skip) {}
            buffer.pos = target;
            return Ok(());
        }

        // Out of window: needs a new Range request. Servers without Range
        // support can only stream forward, so a backward seek is impossible.
        if !buffer.range_supported {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Server does not support Range requests; cannot seek",
            ));
        }
        drop(buffer);

        self.inner.request_range(target).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;
        Ok(())
    }
}

impl MediaSource for HttpStream {
    fn is_seekable(&self) -> bool {
        self.inner.buffer.lock().unwrap().range_supported
    }

    fn byte_len(&self) -> Option<u64> {
        self.inner.buffer.lock().unwrap().total_len
    }
}

impl Drop for HttpStream {
    fn drop(&mut self) {
        self.inner.stop.store(true, Ordering::SeqCst);
        self.inner.cond.notify_all();
        self.inner.resp_cond.notify_all();
        if let Some(handle) = self.inner.handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}
