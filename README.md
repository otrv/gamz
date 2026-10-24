# gamz

gamz is a Rust game engine starter project for Linux and the web. It includes
breakout as an example game. Both platforms use the same game library.

## Build and run

Install Rust with [rustup](https://rustup.rs), then clone the repository:

```sh
git clone https://github.com/otrv/gamz.git
cd gamz
```

Running the desktop game requires Linux, an X11 session, a supported GPU driver,
and an ALSA `default` playback device. On Debian or Ubuntu, install the build
dependencies:

```sh
sudo apt-get install libasound2-dev pkg-config
```

Build and start the game with Cargo. Rustup uses the version pinned in
`rust-toolchain.toml`:

```sh
cargo run -p platform-linux --features linux --bin linux --locked
```

Use `cargo build` in place of `cargo run` to build without launching, or add
`--release` for an optimized build.

## Build for the web

The web platform requires a desktop browser with WebGPU, a keyboard, and HTTPS.
Localhost also works for development. It does not support WebGL or touch controls.

Install the WebAssembly target and the matching packaging tool, then build from
the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version =0.2.129 --locked
./scripts/build-web
```

For development, run `./scripts/build-web --dev`. Both modes write to
`target/web/game/`, so rebuild without `--dev` before publishing or measuring
release performance.

Publish the entire `target/web/game/` directory on a static web host. Keep the
`snippets/` directory and serve `.wasm` files as `application/wasm`. The build
needs no game server or JavaScript package manager.

Keep files requested by the game in the output directory at their relative
paths, such as `target/web/game/assets/level.bin`. Rebuilding preserves added
files. Keep your source assets outside `target/`, because `cargo clean` removes
that directory. Game writes stay in browser storage, not on the static host.

## Make your game

Replace breakout's state and logic in
[crates/game/src/lib.rs](crates/game/src/lib.rs). Implement the
[`platform_api::Game` trait](crates/platform-api/src/game.rs) for
initialization, frame updates, and audio output. Keep your game's assets and
build script in `crates/game`.

For embedding assets, see the [asset-packer guide](crates/asset-packer/README.md).

## Contribute

To contribute changes, install the checking tools and enable the Git hook once
per clone:

```sh
./scripts/setup
git config core.hooksPath .githooks
```

The hook checks affected code when you commit. CI runs the full suite on pull
requests. To run the full suite locally:

```sh
.githooks/pre-commit --all
```
