use std::collections::HashMap;

use color_eyre::eyre::{Context, Result};
use id3::{frame, TagLike};
use m3u::Entry;

use crate::{
    config::{AppConfig, SourceType},
    fetcher::{self, TrackStatus},
    model::{Playlist, Track},
    util,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppIndex {
    /// Maps playlist URL to playlist
    pub playlists: HashMap<String, Playlist>,
    pub deleted: HashMap<String, Vec<Track>>,
    pub removed: HashMap<String, Vec<Track>>,
    pub restricted: HashMap<String, Vec<Track>>,
}

pub struct TrackAction<'a> {
    pub track: &'a Track,
    pub action: Action,
}

macro_rules! act {
    ($track:ident = $action:ident) => {
        TrackAction {
            track: $track,
            action: Action::$action,
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    /// This track was added to the playlist
    Added,

    /// This track was removed from the playlist but still exists on SoundCloud
    Removed,
    /// This track had been removed from the playlist but was added back
    Unremoved,

    /// This track was deleted from SoundCloud
    Deleted,
    /// This track had been deleted from SoundCloud but was added back
    Undeleted,

    /// This track was geo restricted
    Restricted,
    /// This track had been geo restricted but is not anymore
    Unrestricted,
}

impl Action {
    pub fn necessary_operations(&self) -> Vec<Operation> {
        use Action::*;
        use Operation as O;

        match self {
            Added => vec![O::Download, O::AddToPlaylist],
            Removed => vec![O::RemoveFromPlaylist],
            Unremoved => vec![O::AddToPlaylist],
            _ => vec![O::AddMetadataMarker(*self)],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Operation {
    /// Add a message to the Comment metadata stating the state of this track
    AddMetadataMarker(Action),
    /// Add the track to the playlist with the given ID
    AddToPlaylist,
    /// Remove the track from the playlist with the given ID
    RemoveFromPlaylist,
    /// Download the track
    Download,
}

impl Operation {
    #[instrument(skip(self, playlist))]
    pub fn perform(self, track: &Track, playlist: &Playlist) -> Result<()> {
        match self {
            Self::Download => fetcher::ensure_track_downloaded(track)?,
            Self::AddMetadataMarker(state) => {
                trace!("adding metadata marker for {:?}", state);

                let handle = track.as_handle();

                let mut metadata = id3::Tag::read_from_path(&handle.track_path)
                    .wrap_err("failed to read track metadata to write new state to")?;

                let msg = format!(
                    "This track was {}. {} ({})",
                    match state {
                        Action::Added => "added to the playlist",
                        Action::Removed => "removed from the playlist",
                        Action::Unremoved => "added back to the playlist",
                        Action::Deleted => "deleted from SoundCloud",
                        Action::Undeleted => "added back to SoundCloud",
                        Action::Restricted => "geo restricted",
                        Action::Unrestricted => "no longer geo restricted",
                    },
                    playlist.title,
                    playlist.id,
                );

                metadata.add_frame(frame::Comment {
                    lang: "en".to_owned(),
                    text: msg.clone(),
                    description: msg,
                });

                metadata
                    .write_to_path(&handle.track_path, id3::Version::Id3v24)
                    .wrap_err("failed to write track metadata")?;
            }
            Self::AddToPlaylist => {
                trace!("adding track to playlist");

                let playlist_handle = playlist.as_handle();
                let track_handle = track.as_handle();

                let mut playlist = if !playlist_handle.m3u_path.exists() {
                    vec![]
                } else {
                    let mut reader = m3u::Reader::open(&playlist_handle.m3u_path)?;
                    reader.entries().collect::<Result<Vec<_>, _>>()?
                };

                if playlist.iter().any(|e| {
                    matches!(e, Entry::Path(p) if p == &track_handle.relative_to_playlist_dir())
                }) {
                    warn!("track is already in playlist!");
                    return Ok(());
                }

                trace!("playlist has {} entries", playlist.len());

                playlist.push(m3u::path_entry(&track_handle.relative_to_playlist_dir()));

                let mut file = std::fs::File::create(&playlist_handle.m3u_path)?;
                let mut writer = m3u::Writer::new(&mut file);

                trace!("writing entries");
                for entry in playlist {
                    writer.write_entry(&entry)?;
                }
            }
            Self::RemoveFromPlaylist => {
                let playlist_handle = playlist.as_handle();
                let track_handle = track.as_handle();

                let mut playlist = if !playlist_handle.m3u_path.exists() {
                    vec![]
                } else {
                    let mut reader = m3u::Reader::open(&playlist_handle.m3u_path)?;
                    reader.entries().collect::<Result<Vec<_>, _>>()?
                };

                trace!("before removal: playlist has {} entries", playlist.len());

                playlist.retain(|e| !matches!(e, Entry::Path(p) if p == &track_handle.relative_to_playlist_dir()));

                trace!("after removal: playlist has {} entries", playlist.len());

                let mut file = std::fs::File::create(&playlist_handle.m3u_path)?;
                let mut writer = m3u::Writer::new(&mut file);

                trace!("writing entries");
                for entry in playlist {
                    writer.write_entry(&entry)?;
                }
            }
        }

        Ok(())
    }
}

fn cmp_ids(t1: &Track, t2: &Track) -> bool {
    t1.id == t2.id
}

impl AppIndex {
    pub fn load() -> Result<Self> {
        trace!("loading index");
        let path = &AppConfig::get().paths.index;

        if path.exists() {
            trace!("index file exists, loading it");
            let index = std::fs::read_to_string(path)?;

            Ok(serde_json::from_str(&index)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = &AppConfig::get().paths.index;

        trace!("saving index to {}", path.display());
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn refresh(&mut self) -> Result<()> {
        trace!("refreshing index");

        for source in &AppConfig::get().sources {
            trace!("updating source: {}", source.url);

            // this is here so an error will occur if a new source type is added
            match source.kind {
                SourceType::SoundCloud => {}
            }

            let manifest = fetcher::fetch_playlist(&source.url)?;

            let (new_tracks, missing_tracks) =
                if let Some(previous_manifest) = self.playlists.get(&source.url) {
                    util::diff_with(&previous_manifest.entries, &manifest.entries, cmp_ids)
                } else {
                    (manifest.entries.iter().collect(), Vec::new())
                };

            info!(
                "{} new tracks, {} tracks unaccounted for",
                new_tracks.len(),
                missing_tracks.len()
            );

            // tracks that were deleted from SoundCloud entirely
            let mut deleted_tracks = Vec::new();
            // tracks that were manually removed from the playlist
            let mut removed_tracks = Vec::new();
            // tracks that became geo restricted
            let mut restricted_tracks = Vec::new();

            for track in missing_tracks {
                match fetcher::fetch_track(&track.url)? {
                    TrackStatus::Available(_) => {
                        // if the track is still available, it was manually removed
                        // from the playlist
                        removed_tracks.push(track);
                    }
                    TrackStatus::Restricted => {
                        // if the track is geo restricted, it was not manually removed
                        // from the playlist, but it is no longer available
                        restricted_tracks.push(track);
                    }
                    TrackStatus::NotFound => {
                        // if the track is not found, it was deleted from SoundCloud
                        deleted_tracks.push(track);
                    }
                }
            }

            info!(
                "{} deleted tracks, {} removed tracks, {} restricted tracks",
                deleted_tracks.len(),
                removed_tracks.len(),
                restricted_tracks.len()
            );

            let (deleted_tracks, undeleted_tracks) =
                if let Some(previously_deleted) = self.deleted.get(&source.url) {
                    util::diff_ref_with(previously_deleted, deleted_tracks, cmp_ids)
                } else {
                    (deleted_tracks, Vec::new())
                };

            info!(
                "{} deleted tracks, {} undeleted tracks",
                deleted_tracks.len(),
                undeleted_tracks.len()
            );

            let (removed_tracks, unremoved_tracks) =
                if let Some(previously_removed) = self.removed.get(&source.url) {
                    util::diff_ref_with(previously_removed, removed_tracks, |t1, t2| t1.id == t2.id)
                } else {
                    (removed_tracks, Vec::new())
                };

            info!(
                "{} removed tracks, {} unremoved tracks",
                removed_tracks.len(),
                unremoved_tracks.len()
            );

            let (restricted_tracks, unrestricted_tracks) =
                if let Some(previously_restricted) = self.restricted.get(&source.url) {
                    util::diff_ref_with(previously_restricted, restricted_tracks, cmp_ids)
                } else {
                    (restricted_tracks, Vec::new())
                };

            info!(
                "{} restricted tracks, {} unrestricted tracks",
                restricted_tracks.len(),
                unrestricted_tracks.len()
            );

            let mut actions = Vec::new();

            actions.extend(deleted_tracks.iter().map(|t| act!(t = Deleted)));
            actions.extend(undeleted_tracks.iter().map(|t| act!(t = Undeleted)));
            actions.extend(removed_tracks.iter().map(|t| act!(t = Removed)));
            actions.extend(unremoved_tracks.iter().map(|t| act!(t = Unremoved)));
            actions.extend(restricted_tracks.iter().map(|t| act!(t = Restricted)));
            actions.extend(unrestricted_tracks.iter().map(|t| act!(t = Unrestricted)));

            // we want the add operations to come last so the downloads are done last.
            // this is done so if there are errors in the code handling track state changes,
            // we will know before we download tons of audio and crash which would throw away all
            // of the progress we made
            actions.extend(new_tracks.iter().map(|t| act!(t = Added)));

            info!("{} actions to handle", actions.len());

            for action in actions {
                debug!(
                    "handling action {:?} on track {}",
                    action.action, action.track.title
                );
                let track = action.track;

                let operations = action.action.necessary_operations();

                for op in operations {
                    trace!("performing operation {:?}", op);
                    op.perform(track, &manifest)?;
                }
            }

            self.deleted.insert(
                source.url.clone(),
                deleted_tracks.into_iter().cloned().collect(),
            );
            self.removed.insert(
                source.url.clone(),
                removed_tracks.into_iter().cloned().collect(),
            );
            self.restricted.insert(
                source.url.clone(),
                restricted_tracks.into_iter().cloned().collect(),
            );
            self.playlists.insert(source.url.clone(), manifest);
        }

        Ok(())
    }
}
