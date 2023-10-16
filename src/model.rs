use std::{fmt, marker::PhantomData, path::PathBuf};

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};

use crate::config::AppConfig;

fn skip_nulls<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct SkipNulls<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for SkipNulls<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("array with nulls")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(elem) = seq.next_element::<Option<T>>()? {
                vec.extend(elem);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_seq(SkipNulls(PhantomData))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    #[serde(deserialize_with = "skip_nulls")]
    pub entries: Vec<Track>,
    #[serde(rename = "original_url")]
    pub url: String,
    #[serde(rename = "playlist_count")]
    pub len: usize,
}

impl Playlist {
    pub fn as_handle(&self) -> PlaylistHandle {
        let mut playlist_definition_path = AppConfig::get().paths.playlists.join(&self.id);
        playlist_definition_path.set_extension("m3u");

        PlaylistHandle {
            m3u_path: playlist_definition_path,
        }
    }
}

#[derive(Debug)]
pub struct PlaylistHandle {
    pub m3u_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTrack<T> {
    pub id: String,
    pub uploader: String,
    pub title: String,
    #[serde(rename = "original_url")]
    pub url: String,
    #[serde(rename = "playlist_index")]
    // this field is null unless this track was part of a playlist manifest
    pub idx: T,
}

/// A Track that is part of a playlist
pub type Track = RawTrack<usize>;
/// A Track that may or may not be part of a playlist
pub type SingleTrack = RawTrack<()>;

impl Track {
    pub fn as_handle(&self) -> TrackHandle {
        let root_dir = AppConfig::get().paths.audio.join(&self.id);
        let track_path = root_dir.join("track.mp3");

        // FIXME: it is not guaranteed that the cover art will be JPEG
        let album_art_path = root_dir.join("cover.jpg");

        TrackHandle {
            root_dir,
            track_path,
            album_art_path,
        }
    }
}

#[derive(Debug)]
pub struct TrackHandle {
    pub root_dir: PathBuf,
    pub track_path: PathBuf,
    pub album_art_path: PathBuf,
}

impl TrackHandle {
    /// Returns the `track_path` relative to the playlist directory.
    pub fn relative_to_playlist_dir(&self) -> PathBuf {
        let stripped = self
            .track_path
            .strip_prefix(&AppConfig::get().paths.root)
            .unwrap();

        PathBuf::from(format!("../{}", stripped.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_nulls() {
        #[derive(Deserialize)]
        struct SkipNulls {
            #[serde(deserialize_with = "skip_nulls")]
            a: Vec<String>,
        }

        let input = r#"{"a":["hello", null, "world"]}"#;
        let expected = vec!["hello", "world"];

        let actual: SkipNulls = serde_json::from_str(input).unwrap();
        assert_eq!(actual.a, expected);
    }

    #[test]
    fn test_track_handles() {
        AppConfig::initialize();

        let track = Track {
            id: "1234567890".to_string(),
            uploader: "uploader".to_string(),
            title: "title".to_string(),
            url: "https://example.com/fakeuser/track-slug".to_string(),
            idx: 0,
        };

        let handle = track.as_handle();

        assert_eq!(
            handle.track_path,
            PathBuf::from("/tmp/ACAD_TESTS/audio/1234567890/track.mp3")
        );
        assert_eq!(
            handle.relative_to_playlist_dir(),
            PathBuf::from("../audio/1234567890/track.mp3")
        );
    }
}
