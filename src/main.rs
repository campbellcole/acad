use std::time::Duration;

use color_eyre::eyre::{Context, Result};
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::{config::AppConfig, index::AppIndex};

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate serde;

pub mod config;
pub mod index;
pub mod m3u;
pub mod model;
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

    let mut index = AppIndex::load()?;

    loop {
        index.refresh()?;
        index.save()?;

        info!("sleeping for 24 hours");
        std::thread::sleep(Duration::from_secs(60 * 60 * 24));
    }
}
