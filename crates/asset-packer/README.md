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
asset paths. Both modes measure the equivalent binary pack size, not the
generated Rust text. Embedded output omits the catalog from the generated file.
Keep the output directory outside the source directory. The command includes
subdirectories and overwrites the selected output file. Other existing files
are left unchanged. Use separate output directories when keeping both modes.
The optional `--mode embedded|external` defaults to `embedded`.

Use UTF-8 relative paths of at most 1,024 bytes. The packer visits at most 8,192
directory entries and rejects symlinks and non-regular files. External packs retain
the full relative paths, including extensions, with `/` as the directory separator.

Embedded mode generates named Rust byte slices. For these names, it removes the last
extension, replaces path separators and hyphens with underscores, and converts
ASCII letters to uppercase. Names must start with an ASCII letter and contain
only ASCII letters, digits, and underscores. Avoid collisions such as `hero.png`
and `hero.bin`, which both produce `HERO`. External mode does not generate
Rust identifiers, so these identifier restrictions do not apply.

## Embed the assets

Include the generated module from `src/lib.rs` or `src/main.rs`:

```rust
#[path = "generated/assets.gen.rs"]
mod baked;
```

Use each asset directly as a `&'static [u8]`:

```rust
let hero_bytes = baked::HERO;
```

The module contains one `pub(crate) static` byte slice per asset. It needs neither
`game-core` nor a separate `.pack` file. Distribute only the executable.
Regenerate older embedded modules and replace pack lookups with direct names.
In the engine, refer to the generated names when needed rather than storing
references in persistent game state.

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

Name lookup is exact and case-sensitive, without path normalization.
Neither the packer nor the reader decodes PNG files.

In the engine, use the platform file-read API and bounded persistent storage.
Handle read completion and errors before parsing the bytes. The pack reader
does not perform file I/O or manage storage. Current Linux file I/O permits
at most 64 MiB per file and 1 MiB per request, independently of packer budgets.

External packs use GPAK version 3, with paths in the catalog. Regenerate older
version-2 packs before using the new reader.
Generated numeric IDs follow sorted relative paths and belong to their pack.
For external replacements, use path lookup rather than retaining numeric IDs.
Replacement packs must still fit the reader budgets and expected asset formats.

## Read selected assets without loading the whole pack

Use `game_core::asset::PackCatalog` to parse only the header, records, and names.
The reader borrows caller storage and allocates no memory. Keep the pack unchanged
throughout loading because the platform opens the path separately for each request.

1. Request `PACK_HEADER_BYTES` bytes at offset zero through the platform's
   `read_file` callback. Match the returned request ID to `ReadCompleted` and
   require `IoStatus::Complete` with the requested byte count.
2. Call `PackCatalog::<16, 1_048_576>::required_bytes(header_bytes)` with exactly
   the header bytes. The returned length includes the header. Check available
   catalog storage before copying the header into it. Read the remaining catalog
   from offset `PACK_HEADER_BYTES`, in chunks that fit the platform's request limit.
3. After copying each successful completion into storage, parse the exact catalog
   prefix and look up the asset:

   ```rust
   let catalog = game_core::asset::PackCatalog::<16, 1_048_576>::from_bytes(catalog_bytes)?;
   let range = catalog.find("hero.png")?;
   ```

4. Check payload storage against `range.len`, then request chunks starting at
   `range.offset`. Copy retained bytes before the receiving update returns.
   An empty asset needs no payload request. Handle queue exhaustion, cancellation,
   failed reads, and premature EOF before exposing an asset as ready.

The byte budget bounds the declared pack size, not just catalog storage. Catalog
validation checks metadata but cannot detect missing or changed payload bytes,
or extra bytes after the declared pack. Use the whole-file `Pack` reader when you
need to validate the exact file length. Persist ranges and storage offsets rather
than borrowed catalog views. Scheduling, storage, and eviction belong to the game.

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
