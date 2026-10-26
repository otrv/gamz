# asset-packer

`asset-packer` packs a directory of files for embedding or loading at runtime.
Embedded mode produces one self-contained Rust module, `assets.gen.rs`.
External mode produces one binary file, `assets.pack`. File contents are
preserved without decoding or conversion.

## Install and run

Install from the repository root:

```sh
cargo install --path crates/asset-packer --locked
```

From your crate's directory, pack your assets into a separate output directory:

```sh
asset-packer assets/ --max-assets 16 --max-bytes 1048576 --output src/generated
```

The limits and output options are required. Set the file limit to at most 4,096
and the pack budget to 16 through 1,073,741,824 bytes, including the catalog and
asset paths. The budget measures binary bytes, not the generated Rust text.
Keep the output directory outside the source directory. The command includes
subdirectories and overwrites the selected output file. Other existing files
are left unchanged. Use separate output directories when keeping both modes.
The optional `--mode embedded|external` defaults to `embedded`.

Use UTF-8 relative paths of at most 1,024 bytes. The packer visits at most 8,192
directory entries and rejects symlinks and non-regular files. Both modes retain
the full relative paths, including extensions, with `/` as the directory separator.

Embedded mode also generates Rust IDs. For these names, it removes the last
extension, replaces path separators and hyphens with underscores, and converts
ASCII letters to uppercase. Names must start with an ASCII letter and contain
only ASCII letters, digits, and underscores. Avoid collisions such as `hero.png`
and `hero.bin`, which both produce `ids::HERO`. External mode does not generate
IDs in Rust, so these identifier restrictions do not apply.

## Embed the assets

The generated module uses `game-core`. Add it as a dependency if needed, then
include the module from `src/lib.rs` or `src/main.rs`:

```rust
#[path = "generated/assets.gen.rs"]
mod baked;
```

Read an asset inside a function that propagates
`game_core::asset::PackError`:

```rust
let pack = baked::EmbeddedPack::from_bytes(baked::PACK_BYTES)?;
let hero_bytes = pack.bytes(baked::ids::HERO)?;
```

The module contains the pack as a Rust byte array. It has no dependency on a
separate `.pack` file. Distribute only the executable.

## Load an external pack

To distribute the pack separately from the executable, select external mode:

```sh
asset-packer assets/ --max-assets 16 --max-bytes 1048576 --output dist --mode external
```

External mode writes only `dist/assets.pack`. No generated Rust file is needed.

Distribute `assets.pack` with the executable. Load its bytes into storage that
outlives the asset views, then pass the exact loaded slice as `loaded_bytes`.
Choose reader budgets for your application and look up the full asset path:

```rust
let pack = game_core::asset::Pack::<16, 1_048_576>::from_bytes(loaded_bytes)?;
let hero_bytes = pack.bytes_by_name("hero.png")?;
```

Name lookup is exact and case-sensitive, without path normalization. It also
works on embedded packs. Neither the packer nor the reader decodes PNG files.

In the engine, use the platform file-read API and bounded persistent storage.
Handle read completion and errors before parsing the bytes. The pack reader
does not perform file I/O or manage storage. Current platform file I/O permits
at most 64 MiB per file and 1 MiB per request, independently of packer budgets.

Both modes use GPAK version 3, with paths in the catalog. Regenerate older
version-2 packs and embedded modules before using the new reader.
Generated numeric IDs follow sorted relative paths and belong to their pack.
For external replacements, use path lookup rather than retaining numeric IDs.
Replacement packs must still fit the reader budgets and expected asset formats.

## Build with Cargo

To generate the pack during builds, add `asset-packer` as a build dependency.
In `build.rs`, call `asset_packer::build(sources, output, max_assets, max_bytes)`
with `&Path` directories, `usize` budgets, and `OUT_DIR` as the output directory.
Propagate the returned `io::Result<()>` and emit `cargo::rerun-if-changed=assets`
to rebuild when the inputs change. Include the generated module with:

```rust
mod baked {
	include!(concat!(env!("OUT_DIR"), "/assets.gen.rs"));
}
```

For external packs, call
`asset_packer::build_with_mode(sources, output, max_assets, max_bytes, asset_packer::OutputMode::External)`
instead. `build(...)` retains embedded output. Copy `assets.pack` from `OUT_DIR`
into your distribution and load it through your application's file I/O.
