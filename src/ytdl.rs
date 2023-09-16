use std::{fmt::Debug, path::Path, process::Stdio};

use color_eyre::{eyre::eyre, Result};
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

pub async fn fetch_manifests(url: &str) -> Result<Vec<OrderedTrackManifest>> {
    let mut cmd = Command::new("yt-dlp");

    cmd.arg("-j").arg(url);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let child = cmd.spawn()?;

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("yt-dlp failed: {:?}", stderr));
    }

    let stdout = String::from_utf8(output.stdout)?;

    trace!("yt-dlp output: {}", stdout);

    let mut tracks = Vec::new();

    for (idx, line) in stdout.lines().enumerate() {
        let track: TrackManifest = serde_json::from_str(line)?;
        tracks.push(OrderedTrackManifest {
            idx,
            manifest: track,
        });
    }

    Ok(tracks)
}

#[instrument]
pub async fn download_track(track: &Track, dest: impl AsRef<Path> + Debug) -> Result<()> {
    debug!("downloading track: {}", track.url);

    let mut cmd = Command::new("yt-dlp");

    cmd.arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("0")
        .arg("-o")
        .arg(
            dest.as_ref()
                .join(format!("{:04}_%(title)s.%(ext)s", track.idx)),
        )
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
