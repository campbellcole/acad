use color_eyre::eyre::Result;

use crate::model::{Playlist, Track};

use super::{Fetcher, SourceDefinition, TrackDownloadStatus, TrackStatus, source_bail};

pub struct YouTube;

impl Fetcher for YouTube {
    fn fetch_playlist(&self, source: &SourceDefinition) -> Result<Playlist> {
        super::fetch_playlist_generic(&source.url, |output| {
            let stderr = String::from_utf8_lossy(&output.stderr);

            let lines = stderr.lines().collect::<Vec<_>>();

            if !lines.iter().all(|line| line.contains("Video unavailable")) {
                source_bail!(stderr);
            }

            warn!(
                "{} songs were not available for unknown reasons! they will be ignored.",
                lines.len()
            );

            Ok(())
        })
    }

    fn fetch_track(&self, track: &Track) -> color_eyre::eyre::Result<TrackStatus> {
        super::fetch_track_generic(&track.url, |output| {
            let stderr = String::from_utf8_lossy(&output.stderr);

            let lines = stderr.lines().collect::<Vec<_>>();

            if lines.len() == 1
                && (lines[0].contains("Video unavailable") || lines[0].contains("Private video"))
            {
                return Ok(TrackStatus::Restricted);
            }

            // giving youtube an invalid URL does not produce a 404, but this is
            // here in case that behavior changes. currently all invalid URLs
            // produce the "Video unavailable" error above.
            if stderr.contains("HTTP Error 404") {
                return Ok(TrackStatus::NotFound);
            }

            source_bail!(stderr)
        })
    }

    fn ensure_track_downloaded(&self, track: &Track) -> Result<TrackDownloadStatus> {
        super::ensure_track_downloaded_generic(track)
    }
}
