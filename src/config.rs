use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::OnceLock,
};

use color_eyre::eyre::{eyre, Context, Result};
use serde_with::{serde_as, DisplayFromStr};

use crate::source::SourceDefinition;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(skip_deserializing)]
    pub paths: Paths,
    pub save_thumbnails: bool,
    /// The value of the `music_directory` option given to MPD. Used to write
    /// playlist files that can actually be read by MPD (MPD does not handle
    /// relative paths correctly, so we have to write the absolute path of
    /// each track according to the filesystem MPD has access to [i.e. we
    /// can't use the Docker volume's path because MPD doesn't see the same fs])
    pub mpd_music_dir: Option<PathBuf>,
    pub sources: Vec<SourceDefinition>,
    pub refresh_cron: Option<Schedule>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Schedule(#[serde_as(as = "DisplayFromStr")] cron::Schedule);

impl Deref for Schedule {
    type Target = cron::Schedule;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Schedule {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Default)]
pub struct Paths {
    /// The directory where the index and config are stored.
    pub root: PathBuf,
    /// The path to the index file.
    pub index: PathBuf,
    /// The directory where the M3U playlist definitions are stored.
    pub playlists: PathBuf,
    /// The directory where audio files are saved.
    pub audio: PathBuf,
}

impl Paths {
    pub fn from_root(data_folder: PathBuf) -> Self {
        let index = data_folder.join("index.json");
        let playlists = data_folder.join("playlists");
        let audio = data_folder.join("audio");

        Self {
            root: data_folder,
            index,
            playlists,
            audio,
        }
    }

    pub fn ensure_all(&self) -> Result<()> {
        if !self.root.exists() {
            std::fs::create_dir_all(&self.root).wrap_err("failed to create data folder")?;
        }

        if !self.playlists.exists() {
            std::fs::create_dir_all(&self.playlists)
                .wrap_err("failed to create playlists folder")?;
        }

        if !self.audio.exists() {
            std::fs::create_dir_all(&self.audio).wrap_err("failed to create audio folder")?;
        }

        Ok(())
    }
}

static INSTANCE: OnceLock<AppConfig> = OnceLock::new();

impl AppConfig {
    #[cfg(test)]
    const TEST_DATA_ROOT: &'static str = "/tmp/ACAD_TESTS";

    #[cfg(test)]
    pub fn initialize() {
        INSTANCE
            .set(AppConfig {
                paths: Paths::from_root(PathBuf::from(Self::TEST_DATA_ROOT)),
                save_thumbnails: false,
                mpd_music_dir: None,
                sources: Vec::new(),
                refresh_cron: None,
            })
            .unwrap();
    }

    pub fn load() -> Result<()> {
        let data_folder =
            std::env::var("ACAD_DATA_FOLDER").wrap_err("failed to get ACAD_DATA_FOLDER")?;
        let data_folder = PathBuf::from(data_folder);

        let config_path = data_folder.join("config.json");

        if !config_path.exists() {
            return Err(eyre!(
                "config.json does not exist in data folder: {:?}",
                config_path
            ));
        }

        let mut instance =
            serde_json::from_str::<AppConfig>(&std::fs::read_to_string(config_path)?)
                .wrap_err("failed to deserialize config.json")?;

        instance.paths = Paths::from_root(data_folder);

        INSTANCE
            .set(instance)
            .map_err(|_| eyre!("attempted to load config twice"))?;

        Ok(())
    }

    pub fn get() -> &'static Self {
        INSTANCE
            .get()
            .expect("attempted to get config before it was loaded")
    }
}
