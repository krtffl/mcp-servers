//! Token-bucket rate limiter for external API calls.

use std::time::Instant;

use tokio::sync::Mutex;

/// Token-bucket rate limiter.
///
/// Refills tokens at a fixed rate. Each `acquire()` call consumes one token.
/// If no tokens are available, the caller waits until one is refilled.
pub struct RateLimiter {
    state: Mutex<RateLimiterState>,
}

struct RateLimiterState {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// - `max_tokens`: burst capacity
    /// - `refill_rate`: tokens per second
    #[must_use]
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            state: Mutex::new(RateLimiterState {
                tokens: max_tokens,
                max_tokens,
                refill_rate,
                last_refill: Instant::now(),
            }),
        }
    }

    /// Acquire one token, waiting if necessary.
    pub async fn acquire(&self) {
        loop {
            let wait_duration = {
                let mut state = self.state.lock().await;
                let now = Instant::now();
                let elapsed = now.duration_since(state.last_refill).as_secs_f64();
                state.tokens = (state.tokens + elapsed * state.refill_rate).min(state.max_tokens);
                state.last_refill = now;

                if state.tokens >= 1.0 {
                    state.tokens -= 1.0;
                    return;
                }

                // Calculate wait time until one token is available
                let deficit = 1.0 - state.tokens;
                std::time::Duration::from_secs_f64(deficit / state.refill_rate)
            };

            tokio::time::sleep(wait_duration).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn immediate_acquire_within_burst() {
        let limiter = RateLimiter::new(3.0, 1.0);

        // Should succeed immediately for 3 tokens (burst capacity)
        limiter.acquire().await;
        limiter.acquire().await;
        limiter.acquire().await;
    }

    #[tokio::test]
    async fn acquire_waits_when_exhausted() {
        let limiter = RateLimiter::new(1.0, 10.0); // 1 token, refill 10/s

        // First acquire is immediate
        limiter.acquire().await;

        // Second should wait ~100ms for refill
        let start = Instant::now();
        limiter.acquire().await;
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() >= 50, "should have waited for refill");
        assert!(elapsed.as_millis() < 500, "should not wait too long");
    }
}
