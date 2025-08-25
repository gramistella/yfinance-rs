// NEW: Retry config & cache control

#[derive(Clone, Debug)]
pub enum Backoff {
    Fixed(std::time::Duration),
    Exponential {
        base: std::time::Duration, // e.g. 200ms
        factor: f64,               // e.g. 2.0
        max: std::time::Duration,  // cap each sleep
        jitter: bool,              // add +/- 50% jitter
    },
}

#[derive(Clone, Debug)]
pub struct RetryConfig {
    pub enabled: bool,
    pub max_retries: u32, // total attempts = max_retries + 1
    pub backoff: Backoff,
    pub retry_on_status: Vec<u16>, // e.g. [408, 429, 500..=599]
    pub retry_on_timeout: bool,    // reqwest::Error::is_timeout()
    pub retry_on_connect: bool,    // reqwest::Error::is_connect()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheMode {
    /// Read from cache if present; write response to cache.
    Use,
    /// Skip read; write fresh response to cache.
    Refresh,
    /// No read, no write.
    Bypass,
}
