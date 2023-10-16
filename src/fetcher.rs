//! Code for fetching manifests and tracks from SoundCloud.
//!
//! Heavily utilizes `yt-dlp`.

use std::{
    fs,
    process::{Command, Stdio},
};

use color_eyre::eyre::{eyre, Context, Result};

use crate::model::{Playlist, SingleTrack, Track};

const GEO_ERR_LINE_1: &str =
    "This video is not available from your location due to geo restriction";
const GEO_ERR_LINE_2: &str = "You might want to use a VPN or a proxy server";

/// Fetch the playlist manifest from the given URL.
#[instrument]
pub fn fetch_playlist(url: &str) -> Result<Playlist> {
    trace!("fetching playlist manifest");

    let mut cmd = Command::new("yt-dlp");

    // -J fetches the whole playlist as a single manifest as opposed
    // to -j which emits each track as a separate line of JSON
    cmd.arg("-J").arg(url);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd.output()?;

    if !output.status.success() {
        // yt-dlp will exit with code 1 if fetching the manifest failed
        // for any single track, even though it will continue to fetch
        // the rest of the tracks. so we can't just return an error here.
        // we have to ensure all lines of stderr are just proxy warnings.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let lines = stderr.lines().collect::<Vec<_>>();

        // check each error and if we find one that is not a proxy warning, throw an error
        if !lines.chunks(2).all(|chunk| {
            chunk.len() == 2
                && chunk[0].contains(GEO_ERR_LINE_1)
                && chunk[1].contains(GEO_ERR_LINE_2)
        }) {
            return Err(eyre!(
                "yt-dlp failed: (stderr: {:?}) (stdout: {:?})",
                stderr,
                stdout
            ));
        }

        warn!(
            "{} songs were not available due to geo restrictions! they will be ignored.",
            lines.len() / 2
        );
    }

    // lossless because we expect yt-dlp to output valid JSON
    // so non-utf8 data should be an error
    let stdout = String::from_utf8(output.stdout)?;

    Ok(serde_json::from_str(&stdout)?)
}

pub enum TrackStatus {
    Restricted,
    Available(SingleTrack),
    NotFound,
}

/// Fetch the track manifest from the given URL. This is useful for
/// determining whether or not a song still exists on SoundCloud.
#[instrument]
pub fn fetch_track(url: &str) -> Result<TrackStatus> {
    trace!("fetching track manifest");

    let mut cmd = Command::new("yt-dlp");

    cmd.arg("-j").arg(url);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        let lines = stderr.lines().collect::<Vec<_>>();

        if lines.len() == 2
            && lines[0].contains(GEO_ERR_LINE_1)
            && lines[1].contains(GEO_ERR_LINE_2)
        {
            return Ok(TrackStatus::Restricted);
        }

        if stderr.contains("HTTP Error 404") {
            return Ok(TrackStatus::NotFound);
        }

        return Err(eyre!(
            "yt-dlp failed: (stderr: {:?}) (stdout: {:?})",
            stderr,
            stdout
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;

    Ok(TrackStatus::Available(serde_json::from_str(&stdout)?))
}

/// Ensure that the given track is downloaded to disk.
#[instrument]
pub fn ensure_track_downloaded(track: &Track) -> Result<()> {
    trace!("ensuring track is downloaded");

    let handle = track.as_handle();
    if handle.track_path.exists() {
        return Ok(());
    }

    fs::create_dir_all(&handle.root_dir).wrap_err("failed to create track directory")?;

    let mut cmd = Command::new("yt-dlp");

    cmd.args([
        // extract audio (not necessary for SoundCloud but will be useful if other services are added)
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Err(eyre!(
            "yt-dlp failed: (stderr: {:?}) (stdout: {:?})",
            stderr,
            stdout
        ))
    } else {
        Ok(())
    }
}
