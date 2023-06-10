#![feature(drain_filter)]

use color_eyre::{eyre::eyre, Result};
use sources::Sources;
use thirtyfour::{DesiredCapabilities, WebDriver};
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::config::AcadConfig;

#[macro_use]
extern crate serde;
#[macro_use]
extern crate tracing;

pub mod config;
pub mod index;
pub mod nonewrap;
pub mod soundcloud;
pub mod sources;
pub mod wait_find;
pub mod ytdl;

pub struct AppContext<'a> {
    pub driver: &'a WebDriver,
    pub config: &'a AcadConfig,
    pub sources: &'a Sources,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .with(ErrorLayer::default())
        .init();

    color_eyre::install()?;

    let config = config::AcadConfig::get()?;

    println!("{:?}", config);

    if !config.data_folder.exists() {
        std::fs::create_dir_all(&config.data_folder)?;
    }

    let sources = Sources::get(&config)?;

    if sources.is_empty() {
        return Err(eyre!("No sources found"));
    }

    let mut caps = DesiredCapabilities::firefox();
    caps.add_firefox_arg("--headless")?;
    let driver = WebDriver::new("http://localhost:4444", caps).await?;

    let mut index = index::ArchiveIndex::load(&config).await?;

    let ctx = AppContext {
        driver: &driver,
        config: &config,
        sources: &sources,
    };

    index.refresh(&ctx).await?;

    driver.quit().await?;

    Ok(())
}
