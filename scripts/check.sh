#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

step() {
    printf '==> %s\n' "$1"
}

targets=(x86_64-unknown-linux-gnu)
unsafe_crates="game-memory"
miri_toolchain="nightly-2026-10-01"
deny_version="0.20.2"

step "cargo fmt"
cargo fmt --all -- --check

step "cargo deny ($deny_version)"
cargo install cargo-deny --version "=$deny_version" --locked
cargo deny --workspace --locked check --deny warnings bans

rustup target add "${targets[@]}"
for target in "${targets[@]}"; do
    step "cargo clippy --target $target"
    cargo clippy --workspace --all-targets --all-features --locked --target "$target"
    for package in platform benchmark-platform; do
        for feature in cli linux; do
            step "cargo clippy ($package, $feature only) --target $target"
            cargo clippy -p "$package" --no-default-features --features "$feature" --all-targets --locked --target "$target"
        done
    done
done

step "cargo test (game-core and game-memory utilities)"
cargo test -p game-core -p game-memory --locked

rustup toolchain install "$miri_toolchain" --profile minimal --component miri,rust-src --no-self-update
for crate in $unsafe_crates; do
    step "miri ($miri_toolchain): $crate"
    cargo "+$miri_toolchain" miri test -p "$crate" --all-features --locked
done

step "all checks passed"
