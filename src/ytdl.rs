use std::{fmt::Debug, path::Path, process::Stdio};

use color_eyre::{eyre::eyre, Result};
use slugify::slugify;
use tokio::process::Command;

use crate::index::Track;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackManifest {
    pub id: String,
    pub uploader: String,
    pub title: String,
    pub original_url: String,
    pub playlist_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderedTrackManifest {
    pub idx: usize,
    pub manifest: TrackManifest,
}

impl OrderedTrackManifest {
    pub fn into_track(self) -> Track {
        Track {
            idx: self.idx,
            id: self.manifest.id,
            artist: self.manifest.uploader,
            track: self.manifest.title,
            url: self.manifest.original_url,
        }
    }

    pub fn as_track(&self) -> Track {
        Track {
            idx: self.idx,
            id: self.manifest.id.clone(),
            artist: self.manifest.uploader.clone(),
            track: self.manifest.title.clone(),
            url: self.manifest.original_url.clone(),
        }
    }
}

#[instrument]
pub async fn fetch_manifests(url: &str) -> Result<Vec<OrderedTrackManifest>> {
    debug!("fetching manifests");

    let mut cmd = Command::new("yt-dlp");

    cmd.arg("-j").arg(url);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let child = cmd.spawn()?;

    let output = child.wait_with_output().await?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        // yt-dlp will exit with code 1 if fetching the manifest failed
        // for any single track, even though it will continue to fetch
        // the rest of the tracks. so we can't just return an error here.
        // we have to ensure all lines of stderr are just proxy warnings.
        let stderr = String::from_utf8_lossy(&output.stderr);

        let lines = stderr.lines().collect::<Vec<_>>();

        // check each error and if we find one that is not a proxy warning, throw an error
        if !lines.chunks(2).all(|chunk| {
            if chunk.len() != 2 {
                // the error message we are looking for is always
                // 2 lines, so if we don't have 2 lines, we know
                // another error happened
                return false;
            }

            chunk[0]
                .contains("This video is not available from your location due to geo restriction")
                && chunk[1].contains("You might want to use a VPN or a proxy server")
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

    let mut tracks = Vec::new();

    for (idx, line) in stdout.lines().enumerate() {
        let track: TrackManifest = serde_json::from_str(line)?;
        tracks.push(OrderedTrackManifest {
            idx,
            manifest: track,
        });
    }

    trace!("fetched {} manifests", tracks.len());

    Ok(tracks)
}

#[instrument]
pub async fn download_track(track: &Track, dest: impl AsRef<Path> + Debug) -> Result<()> {
    let mut title_slug = slugify!(&track.track, max_length = 32);

    if title_slug.is_empty() {
        title_slug = track.id.clone();
    }

    let filename = format!("{}_{}.mp3", track.idx, title_slug);
    let path = dest.as_ref().join(filename);

    if path.exists() {
        debug!("skipping track because we already have it: {}", track.url);
        return Ok(());
    }

    debug!("downloading track: {}", track.url);

    let mut cmd = Command::new("yt-dlp");

    cmd.arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("0")
        .arg("-o")
        .arg(path)
        .arg("--add-metadata")
        .arg(&track.url);

    cmd.stderr(Stdio::piped());

    let child = cmd.spawn()?;

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;

        return Err(eyre!("yt-dlp failed: {:?}", stderr));
    }

    Ok(())
}
