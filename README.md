# gamz

A Rust game engine.

## Getting started

Requires [rustup](https://rustup.rs); the exact toolchain is pinned in `rust-toolchain.toml`.

```sh
rustup toolchain install
git config core.hooksPath .githooks
cargo run -p platform
scripts/check.sh
```

Run from the repository root with an X11 display and a supported GPU driver.

## Asset provenance

The font assets derive from DejaVu Sans. Its DejaVu/Bitstream license is included in `assets/FONT-LICENSE.txt`. The sprite artwork is original to this project.

## Contributing

[AGENTS.md](AGENTS.md) defines the project's design and enforcement policies.
