# gamz

A Rust game engine.

## Getting started

Requires [rustup](https://rustup.rs); the exact toolchain is pinned in `rust-toolchain.toml`.
Building the Linux graphical platform also requires ALSA development files and `pkg-config` (`sudo apt-get install libasound2-dev pkg-config` on Debian/Ubuntu). These are also needed for the Linux all-features checks, but not for CLI-only builds.

```sh
rustup toolchain install
git config core.hooksPath .githooks
cargo run -p platform --features linux --bin linux --locked
scripts/check.sh
```

Run the Linux graphical platform from the repository root with an X11 display, a supported GPU driver, and a working ALSA `default` playback device.
Audio is independent of the window backend. Underruns restart playback; other audio failures terminate the platform.

For headless execution on Linux, run `cargo run -p platform --features cli --bin cli --locked`.
The CLI loads real files relative to its working directory, using the same Linux file service as the graphical host, but does not initialize windowing, graphics, or an audio device.
It writes initialization uploads to stdout first, then accepts one `FrameInput` JSON object per stdin line and writes the returned frame as one JSON line:

```sh
printf '%s\n' '{"dt":{"secs":0,"nanos":16666667},"controller":{"move_right":{"ended":"down","half_transitions":1}}}' | cargo run -q -p platform --features cli --bin cli --locked
```

To drive audio independently of visual updates, send `{"audio":{"sample_rate":48000,"frames":3}}`.
The response contains `sample_rate` and `frames`, an array of signed 16-bit `[left,right]` pairs.
Audio requests may be interleaved with visual-frame requests in the same process.

For controlled runs and retained evidence, use the [verification skill](.agents/skills/verify-game/SKILL.md). Serialization stays in the CLI so the game-side dependency graph remains free of `std` and `alloc`, even when Cargo unifies features in a full workspace build.

## Contributing

[AGENTS.md](AGENTS.md) defines the project's design and enforcement policies.
