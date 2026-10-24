# gamz

Rust game engine boilerplate. Every workspace crate lives in `crates/<name>`.

Inherit the user's global `AGENTS.md`. This file adds project rules and overrides. Do not copy global rules here.

Follow [STYLE_GUIDE.md](STYLE_GUIDE.md) for all code style, architecture, memory, runtime, unsafe-code, and documentation rules. This file covers agent workflow, enforcement maintenance, approvals, and verification.

## Enforcement

Keep each rule at the highest layer that can enforce it. The table identifies the owners, not an additional checklist for rules those tools already enforce.

| Owner | Responsibility |
| --- | --- |
| `rust-toolchain.toml` | Exact Rust, rustfmt, and Clippy versions. |
| Crate dependency graph | Layer direction. `game-core` cannot depend on `game` or the `platform` binary. |
| `platform-api::Game` and generic host entrypoints | Initialization, frame updates, and audio updates through the shared game interface. |
| `game-entry` features and the selected `examples` module | Exactly one game per build. Unselected example modules do not compile into the game. |
| `Cargo.toml` workspace lints | Fatal diagnostics, restrictions on local lint overrides, unsafe-code isolation, Clippy `all` and `pedantic`, placeholder bans, SAFETY contracts, non-terminating loop signatures, explicit lifetimes, and reachable public items. |
| `clippy.toml` | API-shape lints on public items. |
| `Cargo.toml` release profile | Overflow checks in release builds. |
| `deny.toml` | Allowed direct consumers of engine and game crates across the resolved all-features graph, including development and target-specific dependencies. |
| `crates/asset-packer/deny.toml` | No crate dependencies for `asset-packer`, including build, development, and target-specific dependencies. |
| `crates/xtask/src/main.rs` | Deployment targets, `UNSAFE_CRATES`, exact tool pins, prerequisite installation, and locked checks. Checks cover formatting, dependency bans, compatible all-features builds, the renderer without a backend, every game selection with isolated and combined hosts, utility tests, and Miri. |
| `.githooks/pre-commit` and `.github/workflows/ci.yml` | `cargo xtask check` before commits and in CI. |
| `verify-game` helper | CLI-only build and execution. Engine-contract assertions accept game-independent outputs; game-specific workloads and expectations are caller-supplied files. |
| `STYLE_GUIDE.md` | Code style and design rules that need judgment, including memory and architecture boundaries and handwritten lint suppressions that static checks miss. |
| This file | Workflow, configuration correctness, approvals, and verification. |

Configured diagnostics are fatal. Clippy groups stay at `warn` under `forbid(warnings)` so central exceptions remain possible. `forbidden_lint_groups` prevents local overrides through the compiler's compatibility exception. Add individual `nursery` or `restriction` lints only when they give consistent signal. Never enable the whole `restriction` group.

`forbid(warnings)` does not cover default-deny lints or linker warnings. Explicitly forbid default-deny lints configurable on the pinned stable compiler. Exclude nightly-only and compiler-internal lints. `rust_2024_compatibility` covers `dependency_on_unit_never_type_fallback` without naming a lint removed by the Miri toolchain. Forbid `linker_messages` separately.

Keep `non_camel_case_types`, `non_upper_case_globals`, `unused_attributes`, and `clippy::useless_attribute` at `deny`. Serde derives generate allowances for them, so forbidding local suppression would reject generated code. This exception applies throughout the workspace.

`platform-web` copies the workspace lint tables because `wasm-bindgen` generates allowances inside its callers. Only in that crate, use `deny` instead of `forbid` for `warnings`, `long_running_const_eval`, and the configured Clippy restriction lints. Keep all other levels identical to the workspace, including `unsafe_code = "forbid"`, and update the copy when workspace lints change. Handwritten suppressions remain prohibited.

An unsafe crate copies the workspace lint tables, changing only `unsafe_code` to `"allow"`. Update the copy whenever workspace lints change.

Keep `xtask` a runner for standard tools. Do not add custom source checks, duplicate lint policy, or automate checks of repository configuration itself. Review configuration changes by tracing their enforcement path. Check lint inheritance, exact pins, deployment targets, unsafe-crate lint parity, and local and CI coverage. When upgrading Rust, compare the default-deny lint inventory with `rustc -W help`.

Enforcement configuration is trusted code. Checks prevent mistakes, not deliberate edits to the checks themselves. Hooks can be bypassed. Requiring successful CI before integration is a repository-permissions policy, not a local check. Passing checks do not prove runtime behavior.

## Working rules

Resolve concrete correctness and design problems rather than knowingly commit them. Use `/grill-me` for unresolved design decisions. Optional optimizations and hypothetical requirements are not technical debt.

- Run `cargo xtask check` after any change. Work is not done until it passes. Never use `--no-verify`. Install missing prerequisites with `cargo xtask setup`.
- Fix failed checks. If a diagnostic is wrong, stop and propose a central policy change. Do not suppress it locally or distort clear code to satisfy it.
- Before adding crates, dependencies, public APIs, scripts, checks, abstractions, protocols, tests, or documentation, identify the concrete requirement. Compare removal or reuse with the proposed addition and account for its maintenance cost.
- Get approval before creating a crate, changing a crate's public interface, adding a workspace dependency, or adding an external crate dependency.
- Create crates with `cargo new crates/<name>`. Every crate inherits the workspace edition. Every crate except the designated unsafe crates and `platform-web` inherits the workspace lints.
- After the same correction class occurs twice, use `/correct`. Get approval before building a new prevention check.
- Upgrade toolchain pins together with diagnostic fixes or approved central lint changes. Run `cargo xtask setup` and `cargo xtask check`, then commit the upgrade together.
- Enable the hook once per clone with `git config core.hooksPath .githooks`. `.agents/setup` does this in orbs.

For review requests, inspect the scope the user names. Otherwise review against the merge base with `origin/main`, falling back to `main`, including uncommitted and untracked files. Read whole affected functions, types, and modules where bounds or ownership require context. Run the checks and report failures first without duplicating tool diagnostics. Review code and prose against `STYLE_GUIDE.md`. Report violations, unresolved concerns, and enforcement gaps with file locations, the applicable rule, and a concrete fix. Do not edit code during a review-only request.

## Verification

Verify game-layer changes only through the production CLI, with controlled input and complete outputs. Run platform-specific verification only when that specific platform is altered. Game-layer changes alone never trigger graphical, keyboard, window, or device-playback checks. Apply the same boundary to performance measurements and verification-skill maintenance. Static checks still cover every configured target.

Keep the verification skill and its feature map independent of the game implementation. Game-specific inputs and independently derived assertions belong to the current task, outside the reusable skill. The CLI helper has no graphical-host command. Before any separate platform run, name the affected host or renderer code and the platform property being verified. Skill maintenance alone never justifies that run. Source review enforces these scope decisions; protocol success and matching replay are not gameplay correctness.

Follow the repository's `verify-game` skill for commands and evidence. Report unmeasured performance impact as a limitation.
