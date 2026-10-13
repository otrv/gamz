# asset-packer

`asset-packer` embeds a directory of files in your Rust program. It produces
`assets.pack` and a Rust module, `assets.gen.rs`, with named IDs for the files.
File contents are preserved without decoding or conversion.

## Install and run

Install from the repository root:

```sh
cargo install --path crates/asset-packer --locked
```

From your crate's directory, pack your assets into a separate output directory:

```sh
asset-packer assets/ --max-assets 16 --max-bytes 1048576 --output src/generated
```

The three options set the file-count budget, total pack byte budget, and output
directory. The command includes files in subdirectories and overwrites previous
outputs. File paths determine the generated names, so `hero.png` becomes
`ids::HERO`.

## Use the assets

The generated module uses `game-core`. Add it as a dependency if needed, then
include the module from `src/lib.rs` or `src/main.rs`, with both generated files
in `src/generated`:

```rust
#[path = "generated/assets.gen.rs"]
mod baked;
```

Read an asset inside a function that propagates
`game_core::asset::AssetError`:

```rust
let pack = baked::EmbeddedPack::from_bytes(baked::PACK_BYTES)?;
let hero_bytes = pack.bytes(baked::ids::HERO)?;
```

The program embeds the pack at compile time, so the generated files do not need
to be distributed alongside the executable.

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
