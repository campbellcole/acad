use std::{
    fs,
    process::{Command, Output, Stdio},
};

use color_eyre::eyre::{eyre, Context, Result};

use crate::model::{Playlist, SingleTrack, Track, TrackHandle};

pub mod soundcloud;
pub mod youtube;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    SoundCloud,
    YouTube,
}

impl Fetcher for SourceType {
    fn fetch_playlist(&self, source: &SourceDefinition) -> Result<Playlist> {
        match self {
            Self::SoundCloud => soundcloud::SoundCloud.fetch_playlist(source),
            Self::YouTube => youtube::YouTube.fetch_playlist(source),
        }
    }

    fn fetch_track(&self, track: &Track) -> Result<TrackStatus> {
        match self {
            Self::SoundCloud => soundcloud::SoundCloud.fetch_track(track),
            Self::YouTube => youtube::YouTube.fetch_track(track),
        }
    }

    fn ensure_track_downloaded(&self, track: &Track) -> Result<TrackDownloadStatus> {
        match self {
            Self::SoundCloud => soundcloud::SoundCloud.ensure_track_downloaded(track),
            Self::YouTube => youtube::YouTube.ensure_track_downloaded(track),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceDefinition {
    #[serde(rename = "type")]
    pub kind: SourceType,
    pub url: String,
    #[serde(default)]
    pub inactive: bool,
}

#[derive(Debug, Clone)]
pub enum TrackStatus {
    Restricted,
    Available(SingleTrack),
    NotFound,
}

#[derive(Debug, Clone, Copy)]
pub enum TrackDownloadStatus {
    Downloaded,
    AlreadyDownloaded,
}

pub trait Fetcher {
    fn fetch_playlist(&self, source: &SourceDefinition) -> Result<Playlist>;

    fn fetch_track(&self, track: &Track) -> Result<TrackStatus>;

    fn ensure_track_downloaded(&self, track: &Track) -> Result<TrackDownloadStatus>;
}

#[instrument(skip(on_error))]
fn fetch_playlist_generic<F>(url: &str, on_error: F) -> Result<Playlist>
where
    F: FnOnce(&Output) -> Result<()>,
{
    trace!("fetching playlist manifest");

    let mut cmd = Command::new("yt-dlp");

    cmd.args(["-J", url]);

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output()?;

    if !output.status.success() {
        if let Err(err) = on_error(&output) {
            warn!(
                "yt-dlp reported errors, trying to parse manifest anyway: {}",
                err
            );
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(serde_json::from_str(&stdout)?)
}

#[instrument(skip(on_error))]
fn fetch_track_generic<F>(url: &str, on_error: F) -> Result<TrackStatus>
where
    F: FnOnce(&Output) -> Result<TrackStatus>,
{
    trace!("fetching track manifest");

    let mut cmd = Command::new("yt-dlp");

    cmd.args(["-j", url]);

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd.output()?;

    if !output.status.success() {
        match on_error(&output) {
            Ok(status) => return Ok(status),
            Err(err) => {
                warn!(
                    "yt-dlp reported errors, trying to parse manifest anyway: {}",
                    err
                );
            }
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    let track = match serde_json::from_str(&stdout) {
        Ok(track) => track,
        Err(err) => {
            // parsing the track usually fails if the track has been made private or deleted, so
            // we should just skip it to prevent the app from retrying indefinitely
            warn!(json = stdout, "failed to parse track manifest: {}", err);
            return Ok(TrackStatus::NotFound);
        }
    };

    Ok(TrackStatus::Available(track))
}

#[instrument]
fn ensure_track_downloaded_generic(track: &Track) -> Result<TrackDownloadStatus> {
    // currently there is no difference between platforms when it comes to actually
    // downloading tracks but we will keep this for when that inevitably changes.

    trace!("ensuring track is downloaded");

    let handle = track.as_handle();
    if handle.track_path.exists() {
        return Ok(TrackDownloadStatus::AlreadyDownloaded);
    }

    fs::create_dir_all(&handle.root_dir).wrap_err("failed to create track directory")?;

    let mut cmd = Command::new("yt-dlp");

    cmd.args([
        // extract audio (not necessary for SoundCloud but is for YouTube)
        "-x",
        //
        "--audio-format",
        "mp3",
        "--audio-quality",
        "0",
        // if the URL contains a reference to a playlist, do NOT download the whole playlist
        "--no-playlist",
        "--add-metadata",
        "--write-thumbnail",
    ]);

    cmd.arg("-o").arg(&handle.track_path);

    cmd.arg("-o").arg(format!(
        "thumbnail:{}",
        handle.root_dir.join("cover.%(ext)s").display()
    ));

    cmd.arg(&track.url);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd.output()?;

    if !output.status.success() {
        source_bail!(output);
    }

    debug!("track downloaded, converting thumbnail to JPG");

    if let Err(err) = convert_thumbnail(&handle) {
        warn!("failed to convert thumbnail to JPG: {}", err);
    }

    Ok(TrackDownloadStatus::Downloaded)
}

fn convert_thumbnail(handle: &TrackHandle) -> Result<()> {
    // convert the downloaded thumbnail from whatever file format it's in to JPG
    let dir = fs::read_dir(&handle.root_dir)?;

    let thumbnail = dir.filter_map(Result::ok).find(|e| {
        e.path().file_stem().map(|s| s == "cover").unwrap_or(false)
    }).ok_or_else(|| eyre!("could not locate thumbnail after download. it should have been downloaded as well."))?;

    trace!("found thumbnail: {}", thumbnail.path().display());

    if thumbnail.path().extension().is_some_and(|ext| ext == "jpg") {
        trace!("thumbnail is already in JPG format, skipping conversion");
        // we have to exit here otherwise the file will be deleted because
        // the extension didn't change (and there was no new file created)
        return Ok(());
    }

    let img = image::open(thumbnail.path())?;

    img.save(&handle.album_art_path)?;

    trace!(
        "wrote new thumbnail to {}, deleting old thumbnail...",
        handle.album_art_path.display()
    );

    fs::remove_file(thumbnail.path())?;

    Ok(())
}

macro_rules! source_bail {
    ($output:ident) => {
        ::color_eyre::eyre::bail!("yt-dlp failed: {:?}", $output)
    };
}

pub(crate) use source_bail;
