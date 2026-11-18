# replaycheck

Run solution recordings against a levelset explicitly:

```sh
cargo run -p replaycheck -- levelsets/pocket-wonders levelsets/pocket-wonders/replays
```

The first argument is a levelset directory (containing `index.json`) or the index
JSON itself. Direct levels and relative level references are supported. The second
argument is a directory of replay JSON files, matched by their `level_name` field
against the level's name, independently of filenames. Matching is exact and level
names must be unique.

Missing names or levels warn and skip. A matched replay passes only when the game
emits `GameOver(LevelComplete)`; recorded ticks, steps and bonks are not checked.
Failures, invalid files and loading errors return a nonzero exit status. The
summary lists passed, failed and skipped recordings and warns if none were checked.
These recordings are not automatically run by `cargo test`.
