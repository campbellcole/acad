use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::{
    config::AppConfig,
    index::AppIndex,
    retry::{RetryOptions, RetryPolicy, retry_options_with},
};

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate serde;

pub mod config;
pub mod index;
pub mod m3u;
pub mod model;
pub mod retry;
pub mod source;
pub mod util;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(false),
        )
        .with(EnvFilter::from_default_env())
        .with(ErrorLayer::default())
        .init();

    color_eyre::install()?;

    ctrlc::set_handler(|| {
        info!("received termination signal, exiting");
        if AppIndex::is_refreshing() {
            warn!("index is currently refreshing! all progress except downloads will be lost");
        }
        std::process::exit(0);
    })
    .unwrap();

    trace!("initialized, loading config");

    AppConfig::load().wrap_err("failed to load AppConfig")?;
    AppConfig::get().paths.ensure_all()?;

    let timezone = AppConfig::get().timezone();
    let now = || Utc::now().with_timezone(&timezone);

    let next_refresh = || {
        AppConfig::get()
            .refresh_cron
            .as_ref()
            .and_then(|sched| sched.upcoming(timezone).next())
            .unwrap_or_else(|| now() + chrono::Duration::try_hours(24).unwrap())
    };

    let mut index = AppIndex::load()?;

    const RETRY_OPTIONS: RetryOptions = RetryOptions::new().with_policy(RetryPolicy::Immediate);

    loop {
        retry_options_with(RETRY_OPTIONS, || index.refresh(), "failed to refresh")?;

        let now = now();
        let next = next_refresh();
        debug!("next refresh at: {:?}", next);

        let sleep_duration = next.signed_duration_since(now);
        info!("sleeping for {}", sleep_duration);

        std::thread::sleep(sleep_duration.to_std()?);
    }
}
