# gamz

A Rust game engine.

## Getting started

Requires [rustup](https://rustup.rs); the exact toolchain is pinned in `rust-toolchain.toml`.

```sh
rustup toolchain install
git config core.hooksPath .githooks
cargo run -p gamz
scripts/check.sh
```

## Code style

[AGENTS.md](AGENTS.md) defines the code style and maps every rule to the layer that enforces it: compiler and Clippy policy, `scripts/check.sh` (run by the pre-commit hook and CI), and the `/gamz-review` skill for rules that need judgment.
