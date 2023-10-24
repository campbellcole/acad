use std::fs;

use color_eyre::eyre::Result;

use crate::model::Playlist;

#[instrument(skip(playlist))]
pub fn write_playlist(playlist: &Playlist) -> Result<()> {
    trace!("writing playlist {:?} ({})", playlist.title, playlist.id);

    let playlist_handle = playlist.as_handle();

    if playlist_handle.m3u_path.exists() {
        trace!("deleting old playlist definition");
        fs::remove_file(&playlist_handle.m3u_path)?;
    }

    let mut sorted = playlist.entries.iter().collect::<Vec<_>>();
    sorted.sort_by(|t1, t2| t1.idx.cmp(&t2.idx));

    let contents = playlist
        .entries
        .iter()
        .map(|track| {
            track
                .as_handle()
                .playlist_entry_path()
                .to_string_lossy()
                .to_string()
                + "\n"
        })
        .collect::<String>();

    trace!("writing playlist definition");
    fs::write(&playlist_handle.m3u_path, contents)?;

    Ok(())
}
