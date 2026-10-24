# gamz

gamz is a Rust game engine boilerplate with breakout as its only bundled game.
Replace breakout in `crates/game` with your own game and adapt the engine to
your project. All platforms link the same game library directly.

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
playback device. Press Space to launch the ball and use A and D to move the
paddle. After a win or loss, press Space to reset. Press Escape to exit.

Use `cargo build` in place of `cargo run` to build without launching, or add
`--release` for an optimized build.

## Build for the web

The web platform requires a desktop browser with WebGPU, a keyboard, and HTTPS.
Localhost also works for development. It does not support WebGL or touch controls.

Build your game from the repository root:

```sh
./scripts/setup
./scripts/build-web
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
and releases held keys. Click **Resume** to continue. Use the same breakout
controls as on Linux, except Escape does not close the browser page.

For files loaded through `PlatformApi::load_entire_file`, copy them into the
output directory and list their relative paths in `assets.json`, for example
`["assets/level.bin"]`. The browser loads these files before game initialization.
Rebuilding preserves this manifest and added files. Keep your source assets
outside `target/`, because `cargo clean` removes that directory.

The manifest accepts at most 256 files, with a total budget of 64 MiB and a
per-file limit of 16 MiB plus 8 bytes. Use relative paths without empty components,
`.` or `..` components, backslashes, colons, nulls, `%`, `?`, or `#`.

## Make your game

Replace breakout's state and logic in
[crates/game/src/lib.rs](crates/game/src/lib.rs). Keep `GameState` as the library's
entry type and implement the [Game trait](crates/platform-api/src/lib.rs) for
initialization, frame updates, and audio output. Keep your game's assets and
build script in `crates/game`. There is no game-selection flag or registry.

For embedding assets, see the [asset-packer guide](crates/asset-packer/README.md).

## Check changes

Install the development tools with `./scripts/setup`. Run the pre-commit hook
to check affected crates and their consumers:

```sh
.githooks/pre-commit
```

The hook reads staged, unstaged, and untracked changes. It skips Rust checks for
README and documentation edits. Shared configuration, policy changes, and
unknown paths trigger full checks. Cargo checks the working tree, not an isolated
copy of the staged files.

To run every check, use the same command as CI:

```sh
.githooks/pre-commit --all
```

See [AGENTS.md](AGENTS.md) for project policies and
[verify-game](.agents/skills/verify-game/SKILL.md) for runtime verification.
