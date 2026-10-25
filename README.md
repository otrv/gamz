# gamz

gamz is a Rust game engine starter project for Linux. It includes
breakout as an example game.

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
