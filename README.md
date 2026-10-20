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

## Build for the web

The web platform requires a desktop browser with WebGPU, a keyboard, and HTTPS.
Localhost also works for development. It does not support WebGL or touch controls.

Build your game from the repository root:

```sh
cargo xtask setup
cargo xtask web game
```

Publish the entire `target/web/game/` directory on a static web host. Keep the
`snippets/` directory and serve `.wasm` files as `application/wasm`. The build
includes assets embedded by `asset-packer`. It needs no game server, JavaScript
package manager, or cross-origin isolation headers.

To embed the build in another page, set the iframe URL to its published location:

```html
<iframe
	src="https://games.example.com/my-game/"
	title="My game"
	allow="autoplay"
	style="width: 100%; aspect-ratio: 16 / 9; border: 0"
></iframe>
```

Click **Play** to enable audio and keyboard input. Losing focus pauses the game
and releases held keys. Click **Resume** to continue. The starter game displays
a black canvas. To build an example, replace `game` with a selection from
`crates/game-entry/Cargo.toml`, such as `example-breakout`.

For files loaded through `PlatformApi::load_entire_file`, copy them into the
output directory and list their relative paths in `assets.json`, for example
`["assets/level.bin"]`. The browser loads these files before game initialization.
Rebuilding preserves this manifest and added files. Keep your source assets
outside `target/`, because `cargo clean` removes that directory.

The manifest accepts at most 256 files, with a total budget of 64 MiB and a
per-file limit of 16 MiB plus 8 bytes. Use relative paths without empty components,
`.` or `..` components, backslashes, colons, nulls, `%`, `?`, or `#`.

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
