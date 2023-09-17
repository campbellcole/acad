# acad (Amber-Cast Archive Daemon)

A music archival daemon (currently only supports SoundCloud). This daemon tracks playlists as they change, and will keep any songs which were removed from the playlist or deleted from SoundCloud entirely (both of these categories are stored separately). Useful for those whom follow small artists who delete their music frequently.

**This program is in an experimental state. Everything should work but I wouldn't count on it.**

## Usage

This program is designed to be run in a Docker container alongside a [`geckodriver`](https://github.com/mozilla/geckodriver) Docker container (such as [`instrumentisto/geckodriver`](https://hub.docker.com/r/instrumentisto/geckodriver)). It may also be possible to run this program using a systemd service or `screen` session but this is not currently supported.

### Building

As of now, there are no automated builds for this image. As such, you must build and load this image yourself by cloning the repo and running `docker build -t acad .` (please use buildx).

### Docker Compose

To use the Docker image with Docker Compose, copy the [`docker-compose.yml file`](/docker/docker-compose.yml) to your server and change the `/PATH/FOR/INDEX/AND/MUSIC` path to the path/volume in which you would like the data to be stored. When you run the program later on, this hierarchy will be created:

```
<root>                              # bound to /data in example compose file
├─sources.json                      # you provide this, see below
├─index.json
└┬playlists/
 └┬<playlist id...>/
  ├─<song idx>_<song slug>.mp3
  ├─removed/                        # contains same filenames as above
  └─deleted/                        # ditto
```

### Providing sources

You must store a `sources.json` file in the data folder before starting the container. The file should contain the following structure:

```json
[
  {
    "source_type": "soundcloud",
    "url": "<soundcloud playlist url>"
  },
  {
    // ...
  }
]
```

A description of the fields can be seen below.

| Field         | Description                                                                                                                                                                                                                                                                           |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `source_type` | The source type. Possible values: `soundcloud`                                                                                                                                                                                                                                        |
| `url`         | The URL of the SoundCloud playlist. If the playlist is private, use the sharable URL without any of the URL parameters. For example, `https://soundcloud.com/yourusername/yourplaylist/s-XXXXXXXXXXX`. The version with URL parameters may still work, but using it is not supported. |

Once you have your `sources.json` file prepared and in your data folder, start the program with `docker compose up -d`.

### Logging

Since this program is still in the early stages, it may be useful to `docker watch -f acad` an ensure it gets going properly. You must set `RUST_LOG="acad=trace,info"` in the Compose file as well. The basic order of operations you will see is this:

- iterate each playlist and get it's ID using geckodriver
- iterate them again and for each one:
  - fetch a list of tracks
  - check if we have any songs that are now missing from the playlist
    - if the song still exists on soundcloud, put in `removed`
    - if not, put in `deleted`
  - download any songs we don't have yet
- testing

## Future plans

This program most likely does not actually need geckodriver to accomplish what it's doing. The reason it uses geckodriver is because I got halfway through essentially reimplementing youtube-dl with geckodriver before I realized I was doing that and patched `yt-dlp` into the old geckodriver code.

I would also like to add support for YouTube playlists eventually, and I want to make some abstraction that makes implementing different sources easier.

In the long term I also hope to turn this into an MPD server that transparently handles archiving and essentially acts as a bridge between SoundCloud and MPD. This program already saves metadata and has proven effective at providing files for an external MPD server.
