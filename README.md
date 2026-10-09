# gamz

A Rust game engine.

## Getting started

Requires [rustup](https://rustup.rs); the exact toolchain is pinned in `rust-toolchain.toml`.

```sh
rustup toolchain install
git config core.hooksPath .githooks
cargo run -p platform --features linux --bin linux --locked
scripts/check.sh
```

Run from the repository root with an X11 display and a supported GPU driver.

For headless execution, run `cargo run -p platform --features cli --bin cli --locked`.
The CLI loads real files relative to its working directory, using the same Linux file service as the graphical host, but does not initialize windowing or graphics.
It writes initialization uploads to stdout first, then accepts one `FrameInput` JSON object per stdin line and writes the returned frame as one JSON line:

```sh
printf '%s\n' '{"dt":{"secs":0,"nanos":16666667},"controller":{"move_right":{"ended":"down","half_transitions":1}}}' | cargo run -q -p platform --features cli --bin cli --locked
```

For controlled runs and retained evidence, use the [verification skill](.agents/skills/verify-game/SKILL.md). Serialization stays in the CLI so the game-side dependency graph remains free of `std` and `alloc`, even when Cargo unifies features in a full workspace build.

## Contributing

[AGENTS.md](AGENTS.md) defines the project's design and enforcement policies.
