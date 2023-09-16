use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{Local, NaiveDateTime};
use color_eyre::{eyre::eyre, Result};
use futures::future::join_all;
use tokio::fs;
use walkdir::WalkDir;

use crate::{config::AcadConfig, nonewrap::Nonewrap, soundcloud, ytdl, AppContext};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ArchiveIndex {
    pub playlists: Vec<Playlist>,
    pub last_updated: NaiveDateTime,
}

impl ArchiveIndex {
    pub async fn load(config: &AcadConfig) -> Result<Self> {
        let path = config.data_folder.join("index.json");

        if path.exists() {
            let index = fs::read_to_string(path).await?;

            let index: Self = serde_json::from_str(&index)?;

            Ok(index)
        } else {
            Ok(Self::default())
        }
    }

    async fn save(&self, ctx: &AppContext<'_>) -> Result<()> {
        let path = ctx.config.data_folder.join("index.json");

        let index = serde_json::to_string_pretty(self)?;

        fs::write(path, index).await?;

        Ok(())
    }

    pub async fn refresh(&mut self, ctx: &AppContext<'_>) -> Result<()> {
        info!("refreshing index");
        let mut new_playlists = Vec::new();

        for source in ctx.sources.iter() {
            let source_id = soundcloud::get_playlist_id(ctx, &source.url).await?;

            if !self.playlists.iter().any(|p| p.id == source_id) {
                new_playlists.push(Playlist {
                    id: source_id,
                    url: source.url.clone(),
                    tracks: Vec::new(),
                    removed_tracks: Vec::new(),
                    deleted_tracks: Vec::new(),
                });
            }

            // if we don't do this we get rate limited
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        debug!("{} new playlists", new_playlists.len());

        self.playlists.extend(new_playlists);

        for playlist in self.playlists.iter_mut() {
            debug!("updating playlist: {}", playlist.url);
            let manifest = ytdl::fetch_manifests(&playlist.url).await?;
            trace!("manifest contains {} tracks", manifest.len());

            let new_to_old = manifest
                .into_iter()
                .map(|m| {
                    let corresp = playlist
                        .tracks
                        .iter()
                        .find(|t| t.url == m.manifest.original_url);
                    (m, corresp)
                })
                .collect::<HashMap<_, _>>();

            let old_to_new = playlist.tracks.iter().map(|t| {
                let corresp = new_to_old
                    .iter()
                    .find(|(_, old)| old.as_ref().map(|t| t.url.as_str()) == Some(t.url.as_str()));
                (t, corresp)
            });

            // if a track has no corresponding old track, it's new
            let new_tracks = new_to_old
                .iter()
                .filter_map(|(m, old)| {
                    if old.is_none() {
                        Some(m.as_track())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            // if a track has no corresponding new track, it's deleted
            // note this does not discriminate between removed from the playlist
            // and deleted from the source. we will check for that case manually later
            let missing_tracks = old_to_new
                .filter_map(
                    |(t, new)| {
                        if new.is_none() {
                            Some(t.clone())
                        } else {
                            None
                        }
                    },
                )
                .collect::<Vec<_>>();

            // get the status of the missing tracks
            let missing_tracks_and_status = missing_tracks.into_iter().map(|t| async {
                let res = soundcloud::get_track_id(ctx, &t.url).await;
                // more rate limiting
                tokio::time::sleep(Duration::from_secs(3)).await;
                (t, res)
            });

            let missing_tracks = join_all(missing_tracks_and_status).await;

            // split the missing tracks into deleted and removed
            // if the webdriver failed to load the track page it's because the artist deleted it
            // if the webdriver loaded the track page but the track was not found it's because
            // the track was removed from the playlist
            let (deleted_tracks, removed_tracks): (Vec<_>, Vec<_>) = missing_tracks
                .into_iter()
                .partition(|(_, res)| res.is_err());

            debug!(
                "{}:\n\t{} deleted tracks\n\t{} removed tracks\n\t{} new tracks",
                playlist.url,
                deleted_tracks.len(),
                removed_tracks.len(),
                new_tracks.len()
            );

            let playlist_dir = ctx
                .config
                .data_folder
                .join("playlists")
                .join(&playlist.id.to_string());

            let deleted_dir = playlist_dir.join("deleted");
            fs::create_dir_all(&deleted_dir).await?;

            // update the status of any deleted tracks
            for (track, _) in deleted_tracks {
                // find the location of the removed track
                let (idx, _) = playlist
                    .tracks
                    .iter()
                    .enumerate()
                    .find(|(_, t)| t.id == track.id)
                    .unwrap();

                // remove the track
                let deleted = playlist.tracks.remove(idx);

                // move the track to the removed folder
                let Some(path) = deleted.find_downloaded(&playlist_dir) else {
                    return Err(eyre!(
                        "could not find downloaded track: {:?}",
                        track
                    ));
                };

                let new_path = deleted_dir.join(path.file_name().unwrap());

                trace!("moving {} to {}", path.display(), new_path.display());
                fs::rename(path, new_path).await?;

                playlist.deleted_tracks.push(deleted);

                // update the indices of the tracks after the deleted track
                for t in playlist.tracks[idx..].iter_mut() {
                    let Some(path) = t.find_downloaded(&playlist_dir) else {
                        return Err(eyre!(
                            "could not find downloaded track: {:?}",
                            track
                        ));
                    };

                    let filename_no_idx = &path.file_name().and_then(|s| s.to_str()).unwrap()[5..];

                    let new_path =
                        playlist_dir.join(format!("{:04}_{}", t.idx + 1, filename_no_idx));

                    fs::rename(path, new_path).await?;

                    t.idx -= 1
                }
            }

            let removed_dir = playlist_dir.join("removed");
            fs::create_dir_all(&removed_dir).await?;

            // remove any removed tracks
            for (track, _) in removed_tracks {
                // find the location of the removed track
                let (idx, _) = playlist
                    .tracks
                    .iter()
                    .enumerate()
                    .find(|(_, t)| t.id == track.id)
                    .unwrap();

                // remove the track
                let removed = playlist.tracks.remove(idx);

                // move the track to the removed folder
                let Some(path) = removed.find_downloaded(&playlist_dir) else {
                    return Err(eyre!(
                        "could not find downloaded track: {:?}",
                        track
                    ));
                };

                let new_path = removed_dir.join(path.file_name().unwrap());

                trace!("moving {} to {}", path.display(), new_path.display());
                fs::rename(path, new_path).await?;

                playlist.removed_tracks.push(removed);

                // update the indices of the tracks after the removed track
                for t in playlist.tracks[idx..].iter_mut() {
                    let Some(path) = t.find_downloaded(&playlist_dir) else {
                        return Err(eyre!(
                            "could not find downloaded track: {:?}",
                            track
                        ));
                    };

                    let filename_no_idx = &path.file_name().and_then(|s| s.to_str()).unwrap()[5..];

                    let new_path =
                        playlist_dir.join(format!("{:04}_{}", t.idx + 1, filename_no_idx));

                    fs::rename(path, new_path).await?;

                    t.idx -= 1
                }
            }

            // download all of the new tracks
            for track in &new_tracks {
                ytdl::download_track(track, &playlist_dir).await?;
            }

            // finally, add the new tracks. these will have the correct idx already
            // because yt-dlp will find only valid tracks and we store the idx
            // based on the order yt-dlp prints them out
            playlist.tracks.extend(new_tracks);

            playlist.tracks.sort_by(|a, b| a.idx.cmp(&b.idx));
        }

        self.last_updated = Local::now().naive_local();

        self.save(ctx).await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub url: String,
    pub id: u64,
    pub tracks: Vec<Track>,
    /// Tracks that were removed from the playlist
    pub removed_tracks: Vec<Track>,
    /// Tracks whose author deleted them
    pub deleted_tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub idx: usize,
    pub id: String,
    pub artist: String,
    pub track: String,
    pub url: String,
}

impl Track {
    pub fn find_downloaded(&self, playlist_dir: impl AsRef<Path>) -> Option<PathBuf> {
        let playlist_dir = playlist_dir.as_ref();

        WalkDir::new(playlist_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.nonewrap())
            .filter(|e| e.file_type().is_file())
            .find_map(|e| {
                let path = e.path();

                let filename = path.file_name().unwrap().to_string_lossy();

                let idx = &filename[0..4];
                let idx = idx.parse::<usize>().unwrap();

                if idx == self.idx {
                    Some(path.to_owned())
                } else {
                    None
                }
            })
    }
}
