use std::path::PathBuf;

use config::{Config, ConfigError};

#[derive(Debug, Deserialize)]
pub struct AcadConfig {
    pub data_folder: PathBuf,
    pub geckodriver_hostname: String,
    pub geckodriver_port: u16,
}

impl AcadConfig {
    pub fn get() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(
                config::Environment::default()
                    .prefix("ACAD")
                    .separator("__"),
            )
            .build()?
            .try_deserialize()
    }
}
