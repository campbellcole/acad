# acad

The **A**mber-**C**ast **A**rchive **D**aemon (acad) is a simple daemon that archives
SoundCloud and YouTube playlists using [yt-dlp](https://github.com/yt-dlp/yt-dlp). This tool will download every song in the playlists given to it
and write `.m3u` playlist definitions for each playlist. The index automatically deduplicates
songs, so if multiple playlists contain the same song, it will only be downloaded once per streaming platform.
The index is intended to be used as a music library for MPD (See [MPD Integration](#mpd-integration)).

This tool also keeps track of changes in the playlist and will update the definitions and index accordingly.
Below is a summary of operations performed in response to playlist changes:

| Change                               | Action                                                                |
| ------------------------------------ | --------------------------------------------------------------------- |
| Song added to playlist               | Song is downloaded and added to playlist definition                   |
| Song removed from playlist           | Song is removed from playlist definition (audio is kept in the index) |
| Song deleted from SoundCloud/YouTube | Song is kept in the playlist definition and index                     |
| Song became geo-restricted           | Song is kept in the playlist definition and index                     |

**In no situation will acad delete an audio file. The point of acad is to make a permanent record of all of your music.**

## Usage

The only supported way to use acad is through Docker:

```sh
docker run -d \
    --name acad \
    --restart unless-stopped \
    -v /path/to/acad/data:/data \
    -e ACAD_DATA_FOLDER=/data \
    ghcr.io/campbellcole/acad:latest

# you can enable logging using the RUST_LOG environment variable. e.g.:
# -e RUST_LOG="acad=trace,info"
```

## Configuration

acad is configured using a JSON file. This file must be at `$ACAD_DATA_FOLDER/config.json` in the Docker container.
The following is an example configuration file:

```jsonc
{
  // save thumbnails for songs. saves the thumbnail as `$ACAD_DATA_FOLDER/audio/<id>/cover.jpg`
  // can be retrieved through MPDs `albumart` command (NOT `readpicture`!)
  "save_thumbnails": true,
  // optional. forces playlist definitions to use absolute paths. if you are having trouble getting MPD
  // to read the playlists, set this option to the exact path given to MPD's `music_directory` option
  "mpd_music_dir": "/path/to/mpd/music_directory",
  // here is where you define your playlists
  "sources": [
    {
      "type": "soundcloud",
      // the URL of the soundcloud playlist. supports private playlists if URL is a private share URL.
      // if you use a private share URL, remove the query parameters from the URL so it looks like this:
      "url": "https://soundcloud.com/artist/sets/playlist-name/s-XXXXXXXXXXX"
    },
    {
      "type": "youtube",
      // the URL of the youtube playlist. supports public and unlisted playlists.
      "url": "https://youtube.com/playlist?list=XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
    }
  ]
}
```

## MPD Integration

acad nicely integrates with MPD. If you do not have an existing MPD installation, you can simply set
MPD's `music_directory` option to `$ACAD_DATA_FOLDER/audio`, as well as MPD's `playlist_directory` option
to `$ACAD_DATA_FOLDER/playlists`. With this setup, your MPD library will be 1:1 with your playlists.

If you already have an MPD library set up, symlink `$ACAD_DATA_FOLDER/audio` to your existing `music_directory`.
You will likely have to change `mpd_music_dir` to make sure the playlist definitions correctly reference the
audio files. See [Configuration](#configuration) for more information.

## License

acad is licensed under the MIT license. See [LICENSE](LICENSE) for more information.
