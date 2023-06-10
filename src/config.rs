use std::path::PathBuf;

use config::{Config, ConfigError};

#[derive(Debug, Deserialize)]
pub struct AcadConfig {
    pub data_folder: PathBuf,
}

impl AcadConfig {
    pub fn get() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(config::Environment::default().prefix("SCD").separator("__"))
            .build()?
            .try_deserialize()
    }
}
