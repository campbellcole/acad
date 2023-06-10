use std::ops::Deref;

use color_eyre::Result;

use crate::config::AcadConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub url: String,
    pub source_type: SourceType,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    SoundCloud,
    YouTube,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct Sources(Vec<Source>);

impl Deref for Sources {
    type Target = Vec<Source>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Sources {
    pub fn get(config: &AcadConfig) -> Result<Self> {
        let sources_path = config.data_folder.join("sources.json");

        let sources_str = std::fs::read_to_string(sources_path)?;

        let sources = serde_json::from_str(&sources_str)?;

        Ok(sources)
    }
}
