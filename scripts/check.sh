#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

step() {
    printf '==> %s\n' "$1"
}

unsafe_crates="game-memory"
miri_toolchain="nightly-2026-10-01"

step "cargo fmt"
cargo fmt --all -- --check

targets="$(awk -F'"' '/^targets[[:space:]]*=/ { for (i = 2; i < NF; i += 2) print $i }' rust-toolchain.toml)"
for target in $targets; do
    step "cargo clippy --target $target"
    cargo clippy --workspace --all-targets --all-features --locked --target "$target"
done

host="$(rustc -vV | sed -n 's/^host: //p')"
if printf '%s\n' $targets | grep -qx "$host"; then
    step "cargo test"
    cargo test --workspace --all-features --locked --target "$host"
else
    step "cargo test (host $host has no platform layer; skipping platform)"
    cargo test --workspace --exclude platform --all-features --locked --target "$host"
fi

rustup toolchain install "$miri_toolchain" --profile minimal --component miri,rust-src --no-self-update
for crate in $unsafe_crates; do
    step "miri ($miri_toolchain): $crate"
    cargo "+$miri_toolchain" miri test -p "$crate" --all-features --locked
done

step "all checks passed"
