//! Sampling/Rate limiting layer for tracing events.
//!
//! Implements a token bucket algorithm to limit the rate of trace events per appender.
//!
//! # Example
//!
//! ```
//! use tracing_declarative::sampling::SamplingWriter;
//!
//! // Wrap stdout with a 100 events/second rate limit
//! let writer = SamplingWriter::new(std::io::stdout, 100);
//! // Or use rate_per_second = 0 to disable limiting
//! let unlimited = SamplingWriter::new(std::io::stdout, 0);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing_subscriber::fmt::writer::MakeWriter;

/// A `MakeWriter` wrapper that applies token-bucket rate limiting.
///
/// When `rate_per_second` is 0, all events pass through (no limiting).
/// Otherwise, the bucket refills once per second and each event consumes
/// one token; events that arrive when the bucket is empty are silently
/// dropped (the write succeeds but outputs nothing).
pub struct SamplingWriter<W> {
    inner: W,
    rate_per_second: u64,
    bucket: Arc<AtomicU64>,
    last_refill: Arc<AtomicU64>,
}

impl<W> SamplingWriter<W> {
    /// Create a new sampling writer wrapping `inner` with the given rate.
    ///
    /// # Example
    ///
    /// ```
    /// use tracing_declarative::sampling::SamplingWriter;
    ///
    /// let writer = SamplingWriter::new(std::io::stdout, 100);
    /// ```
    pub fn new(inner: W, rate_per_second: u64) -> Self {
        Self {
            inner,
            rate_per_second,
            bucket: Arc::new(AtomicU64::new(rate_per_second)),
            last_refill: Arc::new(AtomicU64::new(instant_now_ms())),
        }
    }
}

fn instant_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

impl<'a, W> MakeWriter<'a> for SamplingWriter<W>
where
    W: MakeWriter<'a>,
{
    type Writer = SamplingGuard<W::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        let writer = self.inner.make_writer();
        SamplingGuard {
            inner: writer,
            rate_per_second: self.rate_per_second,
            bucket: self.bucket.clone(),
            last_refill: self.last_refill.clone(),
        }
    }
}

/// The writer produced by `SamplingWriter` on each event.
///
/// If the rate limiter allows the event, writes are forwarded to the
/// inner writer. Otherwise, writes are silently consumed.
pub struct SamplingGuard<W> {
    inner: W,
    rate_per_second: u64,
    bucket: Arc<AtomicU64>,
    last_refill: Arc<AtomicU64>,
}

impl<W> SamplingGuard<W> {
    fn try_acquire(&mut self) -> bool {
        if self.rate_per_second == 0 {
            return true;
        }

        let now = instant_now_ms();
        let last = self.last_refill.load(Ordering::SeqCst);
        let elapsed = now.saturating_sub(last);

        if elapsed >= 1000
            && self
                .last_refill
                .compare_exchange(last, now, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            self.bucket.store(self.rate_per_second, Ordering::SeqCst);
        }

        loop {
            let current = self.bucket.load(Ordering::SeqCst);
            if current == 0 {
                return false;
            }
            if self
                .bucket
                .compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return true;
            }
        }
    }
}

impl<W> std::io::Write for SamplingGuard<W>
where
    W: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if !self.try_acquire() {
            return Ok(buf.len());
        }
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Standalone token-bucket rate limiter (not tied to a writer).
///
/// Useful when you need rate-limiting logic without the `MakeWriter` wrapper.
pub struct RateLimiter {
    rate_per_second: u64,
    bucket: Arc<AtomicU64>,
    last_refill: Arc<AtomicU64>,
}

impl RateLimiter {
    /// Create a rate limiter that allows `rate_per_second` events per second.
    /// A value of 0 disables limiting (all events pass).
    ///
    /// # Example
    ///
    /// ```
    /// use tracing_declarative::sampling::RateLimiter;
    ///
    /// let limiter = RateLimiter::new(50);
    /// assert!(limiter.is_allowed());
    /// ```
    pub fn new(rate_per_second: u64) -> Self {
        Self {
            rate_per_second,
            bucket: Arc::new(AtomicU64::new(rate_per_second)),
            last_refill: Arc::new(AtomicU64::new(instant_now_ms())),
        }
    }

    /// Returns `true` if the current event is allowed under the rate limit.
    ///
    /// # Example
    ///
    /// ```
    /// use tracing_declarative::sampling::RateLimiter;
    ///
    /// let limiter = RateLimiter::new(0); // unlimited
    /// assert!(limiter.is_allowed());
    /// ```
    pub fn is_allowed(&self) -> bool {
        if self.rate_per_second == 0 {
            return true;
        }

        let now = instant_now_ms();
        let last = self.last_refill.load(Ordering::SeqCst);
        let elapsed = now.saturating_sub(last);

        if elapsed >= 1000
            && self
                .last_refill
                .compare_exchange(last, now, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        {
            self.bucket.store(self.rate_per_second, Ordering::SeqCst);
        }

        let current = self.bucket.load(Ordering::SeqCst);
        if current == 0 {
            return false;
        }
        self.bucket.fetch_sub(1, Ordering::SeqCst) > 0
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            rate_per_second: self.rate_per_second,
            bucket: self.bucket.clone(),
            last_refill: self.last_refill.clone(),
        }
    }
}
