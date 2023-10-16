# acad (new version)

## TODO

- Improve the metadata marker system
  - Perhaps add a special field to the metadata called `history` or something to store the state changes
    of a track over time. Currently the history is written directly into a comment and it looks pretty bad
    and overwrites the yt-dlp metadata.
- Finish testing
  - Already tested simple mutations: added, removed, unremoved, and deleted
  - Need to test: undeleted, restricted, unrestricted (not sure if testing any of these is possible though)
- ~~Remake `Dockerfile`~~
- Instead of adding and removing entries from a playlist file, we should generate the playlist
  file all at once so we can control the order of the entries to keep them equal to the source playlist
- Finish writing tests
