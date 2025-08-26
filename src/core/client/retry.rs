/// Specifies the backoff strategy for retrying failed requests.
#[derive(Clone, Debug)]
pub enum Backoff {
    /// Uses a fixed delay between retries.
    Fixed(std::time::Duration),
    /// Uses an exponential delay between retries.
    /// The delay is calculated as `base * (factor ^ attempt)`.
    Exponential {
        /// The initial backoff duration.
        base: std::time::Duration,
        /// The multiplicative factor for each subsequent retry.
        factor: f64,
        /// The maximum duration to wait between retries.
        max: std::time::Duration,
        /// Whether to apply random jitter (+/- 50%) to the delay.
        jitter: bool,
    },
}

/// Configuration for the automatic retry mechanism.
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Enables or disables the retry mechanism.
    pub enabled: bool,
    /// The maximum number of retries to attempt. The total number of attempts will be `max_retries + 1`.
    pub max_retries: u32,
    /// The backoff strategy to use between retries.
    pub backoff: Backoff,
    /// A list of HTTP status codes that should trigger a retry.
    pub retry_on_status: Vec<u16>,
    /// Whether to retry on request timeouts.
    pub retry_on_timeout: bool,
    /// Whether to retry on connection errors.
    pub retry_on_connect: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 4,
            backoff: Backoff::Exponential {
                base: std::time::Duration::from_millis(200),
                factor: 2.0,
                max: std::time::Duration::from_secs(3),
                jitter: true,
            },
            retry_on_status: vec![408, 429, 500, 502, 503, 504],
            retry_on_timeout: true,
            retry_on_connect: true,
        }
    }
}

/// Defines the behavior of the in-memory cache for an API call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheMode {
    /// Read from the cache if a non-expired entry is present; otherwise, fetch from the network
    /// and write the response to the cache. (Default)
    Use,
    /// Always fetch from the network, bypassing any cached entry, and write the new response to the cache.
    Refresh,
    /// Always fetch from the network and do not read from or write to the cache.
    Bypass,
}
