#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

list_files() {
    git ls-files -z --cached --others --exclude-standard -- "$@"
}

step() {
    printf '==> %s\n' "$1"
}

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

step "manifests: every package inherits the workspace lint policy"
list_files 'Cargo.toml' '*/Cargo.toml' | xargs -0 awk '
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

step "cargo check"
cargo check --workspace --all-targets --all-features --locked

step "cargo clippy"
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

step "cargo test"
cargo test --workspace --all-features --locked

step "all checks passed"
