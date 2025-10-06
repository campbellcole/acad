use color_eyre::eyre::Result;

use crate::model::{Playlist, Track};

use super::{Fetcher, SourceDefinition, TrackDownloadStatus, TrackStatus, source_bail};

pub struct SoundCloud;

const GEO_ERR_LINE_1: &str =
    "This video is not available from your location due to geo restriction";
const GEO_ERR_LINE_2: &str = "You might want to use a VPN or a proxy server";

impl Fetcher for SoundCloud {
    fn fetch_playlist(&self, source: &SourceDefinition) -> Result<Playlist> {
        super::fetch_playlist_generic(&source.url, |output| {
            // yt-dlp will exit with code 1 if fetching the manifest failed for
            // any single track, even though it will continue to fetch the rest
            // of the tracks. so we can't just return an error here. we have to
            // ensure all lines of stderr are just proxy warnings.
            let stderr = String::from_utf8_lossy(&output.stderr);

            let lines = stderr.lines().collect::<Vec<_>>();

            // check each error and if we find one that is not a proxy warning,
            // throw an error
            if !lines.chunks(2).all(|chunk| {
                chunk.len() == 2
                    && chunk[0].contains(GEO_ERR_LINE_1)
                    && chunk[1].contains(GEO_ERR_LINE_2)
            }) {
                source_bail!(stderr);
            }

            warn!(
                "{} songs were not available due to geo restrictions! they will be ignored.",
                lines.len() / 2
            );

            Ok(())
        })
    }

    fn fetch_track(&self, track: &Track) -> Result<TrackStatus> {
        super::fetch_track_generic(&track.url, |output| {
            let stderr = String::from_utf8_lossy(&output.stderr);

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

            source_bail!(stderr)
        })
    }

    fn ensure_track_downloaded(&self, track: &Track) -> Result<TrackDownloadStatus> {
        super::ensure_track_downloaded_generic(track)
    }
}
