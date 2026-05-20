DAT Level Extractor
===================

Extract all level data from .dat files and resave them in this project's format.

```
cargo run --bin chipundat -- <INPUT.dat> [OUT_DIR] [-e ENCODING]
```

Where `<INPUT>` is the path to a .dat file and `[OUT_DIR]` is the path to a directory to save the extracted levels to.

The optional `-e` flag specifies the text encoding to use when reading level metadata. If not specified, it defaults to `windows1252`.
