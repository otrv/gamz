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

## Contributing

[AGENTS.md](AGENTS.md) defines the project's design and enforcement policies.
