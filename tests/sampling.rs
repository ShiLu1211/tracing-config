//! Tests for sampling/rate limiting functionality.

use tracing_declarative::sampling::RateLimiter;

#[test]
fn test_rate_limiter_zero_rate_allows_all() {
    let limiter = RateLimiter::new(0);
    assert!(limiter.is_allowed());
    assert!(limiter.is_allowed());
    assert!(limiter.is_allowed());
}

#[test]
fn test_rate_limiter_clone() {
    let limiter = RateLimiter::new(100);
    let cloned = limiter.clone();
    drop(limiter);
    assert!(cloned.is_allowed());
}

#[test]
fn test_rate_limiter_high_rate() {
    let limiter = RateLimiter::new(10000);
    for _ in 0..100 {
        assert!(limiter.is_allowed());
    }
}
