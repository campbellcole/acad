use std::{fmt::Debug, time::Duration};

#[derive(Debug, Clone, Copy, Default)]
pub enum RetryPolicy {
    /// Retry immediately.
    #[default]
    Immediate,
    /// Retry after a delay.
    Delay(Duration),
    /// Retry after a delay, with exponential backoff.
    Exponential(Duration),
}

#[derive(Debug, Clone, Copy)]
pub struct RetryOptions {
    /// The maximum number of retries. If `None`, retries are unlimited.
    pub max_retries: Option<usize>,
    /// The retry policy.
    pub policy: RetryPolicy,
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryOptions {
    pub const fn new() -> Self {
        Self {
            max_retries: None,
            policy: RetryPolicy::Immediate,
        }
    }

    pub const fn with_policy(mut self, policy: RetryPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub const fn from_policy(policy: RetryPolicy) -> Self {
        Self {
            policy,
            ..Self::new()
        }
    }

    pub const fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    pub const fn from_max_retries(max_retries: usize) -> Self {
        Self {
            max_retries: Some(max_retries),
            ..Self::new()
        }
    }
}

pub fn retry_options<T, F, E>(options: RetryOptions, f: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: Debug,
{
    retry_options_with(options, f, "operation failed")
}

pub fn retry_options_with<T, F, E>(
    options: RetryOptions,
    mut f: F,
    msg: impl AsRef<str>,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: Debug,
{
    let msg = msg.as_ref();
    let mut retries = 0;

    loop {
        match f() {
            Ok(value) => return Ok(value),
            Err(err) => {
                error!("{} ({}): {:?}", msg, retries, err);

                if options.max_retries.is_some_and(|max| retries >= max) {
                    return Err(err);
                }

                match options.policy {
                    RetryPolicy::Immediate => {}
                    RetryPolicy::Delay(delay) => {
                        std::thread::sleep(delay);
                    }
                    RetryPolicy::Exponential(delay) => {
                        let delay = delay * 2u32.pow(retries as u32);
                        std::thread::sleep(delay);
                    }
                }

                retries += 1;
            }
        }
    }
}

pub const DEFAULT_OPTIONS: RetryOptions = RetryOptions {
    max_retries: Some(3),
    policy: RetryPolicy::Exponential(Duration::from_secs(1)),
};

pub fn retry<T, F, E>(f: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: Debug,
{
    retry_options(DEFAULT_OPTIONS, f)
}

pub fn retry_with<T, F, E>(f: F, msg: impl AsRef<str>) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: Debug,
{
    retry_options_with(DEFAULT_OPTIONS, f, msg)
}
