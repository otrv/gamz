# asset-packer

Packs a directory of files into a binary pack and a Rust module that embeds it.

## Install and run

Install from the repository checkout:

```sh
cargo install --path crates/asset-packer --locked
```

```sh
asset-packer assets/ --max-assets 16 --max-bytes 1048576 --output src/generated
```

All three flags are required. `--max-assets` limits the file count;
`--max-bytes` limits the total pack size, including its catalog.
The output directory must be outside the source directory. Existing generated
files are overwritten.

## Inputs

The packer includes regular files recursively, preserving their bytes without
decoding them. Empty files and files without extensions are supported. Symlinks
are rejected.

Generated names come from relative paths:

```text
assets/hero.png       -> ids::HERO
assets/music.ogg      -> ids::MUSIC
assets/levels/intro   -> ids::LEVELS_INTRO
```

The final extension is removed, `/` and `-` become `_`, and letters become
uppercase. Names must start with an ASCII letter and contain only ASCII letters,
digits, and underscores. Duplicate names are rejected. Numeric IDs follow sorted
relative-path order and can change between builds.

Limits are 4096 files, 8192 directory entries, 1024 bytes per relative path, and
a pack budget of 16 bytes to 1 GiB.

## Use the output

Keep `assets.gen.rs` and `assets.pack` together. Include the generated module in
a crate that depends on `game-core`:

```rust
#[path = "generated/assets.gen.rs"]
mod baked;
```

Retrieve an asset's bytes:

```rust
let pack = baked::EmbeddedPack::from_bytes(baked::PACK_BYTES)?;
let hero_bytes = pack.bytes(baked::ids::HERO)?;
```

Both generated files are needed at compile time, not at runtime.

## Cargo build integration

Add `asset-packer` as a build dependency. In `build.rs`, call
`asset_packer::build(sources, output, max_assets, max_bytes)` with `&Path`
directories and `usize` limits, using `OUT_DIR` for output. Propagate errors
to fail the build.

Emit `cargo::rerun-if-changed=assets` to track source changes, additions, and
removals. If inputs are generated during the build, watch their original sources
instead and recreate the generated input directory on each build-script run.

```rust
mod baked {
    include!(concat!(env!("OUT_DIR"), "/assets.gen.rs"));
}
```
