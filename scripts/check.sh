#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

list_files() {
    git ls-files -z --cached --others --exclude-standard -- "$@"
}

step() {
    printf '==> %s\n' "$1"
}

unsafe_crates="game-memory"
miri_toolchain="nightly-2026-10-01"

step "comments: only SAFETY contracts may be written as comments"
list_files '*.rs' | xargs -0 awk '
    FNR == 1 { contract = 0 }
    /^[[:space:]]*\/\/ SAFETY: / || /^[[:space:]]*\/\/\/ # Safety$/ { contract = 1; next }
    /^[[:space:]]*(\/\/|\/\*)/ {
        if (!contract) { printf "%s:%d: comment outside a SAFETY contract\n", FILENAME, FNR; failed = 1 }
        next
    }
    { contract = 0 }
    END { exit failed }
' /dev/null

step "suppressions: no inner #![allow] and no #[expect] in any form"
list_files '*.rs' | xargs -0 awk '
    /#!\[[[:space:]]*(cfg_attr\([^]]*[(,[:space:]])?allow[[:space:]]*\(/ ||
    /#!?\[[[:space:]]*(cfg_attr\([^]]*[(,[:space:]])?expect[[:space:]]*\(/ {
        printf "%s:%d: local lint suppression\n", FILENAME, FNR
        failed = 1
    }
    END { exit failed }
' /dev/null

step "manifests: every package except the unsafe crates inherits the workspace lint policy"
excluded=()
for crate in $unsafe_crates; do
    excluded+=(":!crates/$crate/Cargo.toml")
done
list_files 'Cargo.toml' '*/Cargo.toml' "${excluded[@]}" | xargs -0 awk '
    function finish() {
        if (file != "" && package && !inherits) {
            printf "%s: missing [lints] workspace = true\n", file
            failed = 1
        }
    }
    FNR == 1 { finish(); file = FILENAME; package = 0; inherits = 0; table = "" }
    /^\[/ { table = $0 }
    table == "[package]" { package = 1 }
    table == "[lints]" && /^workspace[[:space:]]*=[[:space:]]*true[[:space:]]*$/ { inherits = 1 }
    END { finish(); exit failed }
' /dev/null

step "cargo fmt"
cargo fmt --all -- --check

targets="$(awk -F'"' '/^targets[[:space:]]*=/ { for (i = 2; i < NF; i += 2) print $i }' rust-toolchain.toml)"
if [ -z "$targets" ]; then
    printf 'rust-toolchain.toml lists no deployment targets\n'
    exit 1
fi
for target in $targets; do
    step "cargo clippy --target $target"
    cargo clippy --workspace --all-targets --all-features --locked --target "$target" -- -D warnings
done

host="$(rustc -vV | sed -n 's/^host: //p')"
if printf '%s\n' $targets | grep -qx "$host"; then
    step "cargo test"
    cargo test --workspace --all-features --locked --target "$host"
else
    step "cargo test (host $host has no platform layer; skipping platform)"
    cargo test --workspace --exclude platform --all-features --locked --target "$host"
fi

for crate in $unsafe_crates; do
    step "miri ($miri_toolchain): $crate"
    rustup toolchain install "$miri_toolchain" --profile minimal --component miri,rust-src --no-self-update >/dev/null 2>&1
    cargo "+$miri_toolchain" miri test -p "$crate" --all-features --locked
done

step "all checks passed"
