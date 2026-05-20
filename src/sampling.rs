//! Sampling/Rate limiting layer for tracing events.
//!
//! Implements a token bucket algorithm to limit the rate of trace events per appender.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing_subscriber::fmt::writer::MakeWriter;

pub struct SamplingWriter<W> {
    inner: W,
    rate_per_second: u64,
    bucket: Arc<AtomicU64>,
    last_refill: Arc<AtomicU64>,
}

impl<W> SamplingWriter<W> {
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

pub struct RateLimiter {
    rate_per_second: u64,
    bucket: Arc<AtomicU64>,
    last_refill: Arc<AtomicU64>,
}

impl RateLimiter {
    pub fn new(rate_per_second: u64) -> Self {
        Self {
            rate_per_second,
            bucket: Arc::new(AtomicU64::new(rate_per_second)),
            last_refill: Arc::new(AtomicU64::new(instant_now_ms())),
        }
    }

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
