//! Rate limiting for security-sensitive operations
//!
//! Provides rate limiting to prevent abuse of expensive operations
//! like DID generation.

use crate::error::{DidError, NaviaResult};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Simple rate limiter using a token bucket algorithm
pub struct RateLimiter {
    /// Maximum number of operations allowed in the time window
    max_operations: u32,
    /// Time window for rate limiting
    window: Duration,
    /// Track operations per key
    operations: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    /// Creates a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `max_operations` - Maximum operations allowed in the time window
    /// * `window` - Time window duration
    pub fn new(max_operations: u32, window: Duration) -> Self {
        Self {
            max_operations,
            window,
            operations: Mutex::new(HashMap::new()),
        }
    }

    /// Checks if an operation is allowed for the given key
    ///
    /// # Arguments
    ///
    /// * `key` - Identifier for rate limiting (e.g., "did_generation")
    ///
    /// # Returns
    ///
    /// Ok(()) if the operation is allowed, or an error if rate limit exceeded
    pub fn check_rate_limit(&self, key: &str) -> NaviaResult<()> {
        let mut operations = self.operations.lock().unwrap();
        let now = Instant::now();

        // Get or create the operation list for this key
        let ops = operations.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove operations outside the time window
        ops.retain(|&instant| now.duration_since(instant) < self.window);

        // Check if we've exceeded the limit
        if ops.len() >= self.max_operations as usize {
            return Err(DidError::RateLimitExceeded {
                details: format!(
                    "maximum {} operations per {:?}",
                    self.max_operations, self.window
                ),
            }
            .into());
        }

        // Record this operation
        ops.push(now);

        Ok(())
    }

    /// Clears all rate limit tracking data
    ///
    /// Useful for testing or resetting limits
    pub fn clear(&self) {
        let mut operations = self.operations.lock().unwrap();
        operations.clear();
    }
}

/// Global rate limiter for DID generation
///
/// Limits DID generation to prevent abuse. The limits are:
/// - Configurable via NaviaConfig (default: 10 DIDs per minute)
/// - Can be adjusted based on security requirements
pub static DID_GENERATION_LIMITER: once_cell::sync::Lazy<RateLimiter> =
    once_cell::sync::Lazy::new(|| {
        let config = crate::core::config::get_config().read();
        let max_operations = config.max_dids_per_minute;
        drop(config); // Release the lock
        RateLimiter::new(max_operations, Duration::from_secs(60))
    });

/// Test-only rate limiter with shorter window for faster tests
#[cfg(test)]
pub static TEST_DID_GENERATION_LIMITER: once_cell::sync::Lazy<RateLimiter> =
    once_cell::sync::Lazy::new(|| RateLimiter::new(3, Duration::from_secs(1)));

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_rate_limiter_allows_operations() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));

        // First 3 operations should succeed
        assert!(limiter.check_rate_limit("test").is_ok());
        assert!(limiter.check_rate_limit("test").is_ok());
        assert!(limiter.check_rate_limit("test").is_ok());

        // 4th operation should fail
        assert!(limiter.check_rate_limit("test").is_err());
    }

    #[test]
    fn test_rate_limiter_window_reset() {
        let limiter = RateLimiter::new(2, Duration::from_millis(100));

        // Use up the limit
        assert!(limiter.check_rate_limit("test").is_ok());
        assert!(limiter.check_rate_limit("test").is_ok());
        assert!(limiter.check_rate_limit("test").is_err());

        // Wait for window to expire
        thread::sleep(Duration::from_millis(150));

        // Should be allowed again
        assert!(limiter.check_rate_limit("test").is_ok());
    }

    #[test]
    fn test_rate_limiter_different_keys() {
        let limiter = RateLimiter::new(1, Duration::from_secs(1));

        // Different keys have separate limits
        assert!(limiter.check_rate_limit("key1").is_ok());
        assert!(limiter.check_rate_limit("key2").is_ok());

        // But each key is limited
        assert!(limiter.check_rate_limit("key1").is_err());
        assert!(limiter.check_rate_limit("key2").is_err());
    }
}
