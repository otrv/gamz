# gamz

gamz is a Rust game engine boilerplate. Start your game here and change the
engine to suit your project.

## Build and run

The desktop platform uses Linux, X11, ALSA, and `wgpu`.
Install [rustup](https://rustup.rs). On Debian or Ubuntu, install the build
dependencies with:

```sh
sudo apt-get install libasound2-dev pkg-config
```

From the repository root, install the pinned Rust toolchain and start the game:

```sh
rustup toolchain install
cargo run -p platform --features linux --bin linux --locked
```

Run it in an X11 session with a supported GPU driver and an ALSA `default`
playback device. The starter game opens a black canvas. Use `cargo build` in
place of `cargo run` to build without launching, or add `--release` for an
optimized build.

## Make your game

Start in [crates/game/src/lib.rs](crates/game/src/lib.rs). `GameState` implements
the [Game trait](crates/platform-api/src/lib.rs), with methods for initialization,
frame updates, and audio output. Replace the starter behavior with your game's
state and logic. `game-entry` selects the game linked into the executable.

For embedding assets, see the [asset-packer guide](crates/asset-packer/README.md).

## Check changes

Install the development tools with `cargo xtask setup`, then run
`cargo xtask check`. See [AGENTS.md](AGENTS.md) for project policies and
[verify-game](.agents/skills/verify-game/SKILL.md) for runtime verification.
