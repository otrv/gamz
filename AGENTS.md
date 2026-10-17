# gamz

Rust game engine boilerplate. Every workspace crate lives in `crates/<name>`.

Inherit the user's global `AGENTS.md`. This file adds project rules and overrides. Do not copy global rules here.

## Priorities

For this project, rank correctness, predictability, and performance in that order. Choose the simplest design that preserves those priorities.

Resolve concrete correctness and design problems rather than knowingly commit them. Use `/grill-me` for unresolved design decisions. Optional optimizations and hypothetical requirements are not technical debt.

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
| This file | Rules that need judgment, configuration correctness, and handwritten lint suppressions that static checks miss. |

Configured diagnostics are fatal. Clippy groups stay at `warn` under `forbid(warnings)` so central exceptions remain possible. `forbidden_lint_groups` prevents local overrides through the compiler's compatibility exception. Add individual `nursery` or `restriction` lints only when they give consistent signal. Never enable the whole `restriction` group.

`forbid(warnings)` does not cover default-deny lints or linker warnings. Explicitly forbid default-deny lints configurable on the pinned stable compiler. Exclude nightly-only and compiler-internal lints. `rust_2024_compatibility` covers `dependency_on_unit_never_type_fallback` without naming a lint removed by the Miri toolchain. Forbid `linker_messages` separately.

Keep `non_camel_case_types`, `non_upper_case_globals`, `unused_attributes`, and `clippy::useless_attribute` at `deny`. Serde derives generate allowances for them, so forbidding local suppression would reject generated code. This exception applies throughout the workspace.

Reject handwritten local lint suppressions, including suppressions of those four lints, allowances for centrally allowed lints, and attributes in unchecked configurations. Third-party macro-generated allowances are not handwritten suppressions.

Keep `xtask` a runner for standard tools. Do not add custom source checks, duplicate lint policy, or automate checks of repository configuration itself. Review configuration changes by tracing their enforcement path. Check lint inheritance, exact pins, deployment targets, unsafe-crate lint parity, and local and CI coverage. When upgrading Rust, compare the default-deny lint inventory with `rustc -W help`.

Enforcement configuration is trusted code. Checks prevent mistakes, not deliberate edits to the checks themselves. Hooks can be bypassed. Requiring successful CI before integration is a repository-permissions policy, not a local check. Passing checks do not prove runtime behavior.

## Working rules

- Run `cargo xtask check` after any change. Work is not done until it passes. Never use `--no-verify`. Install missing prerequisites with `cargo xtask setup`.
- Fix failed checks. If a diagnostic is wrong, stop and propose a central policy change. Do not suppress it locally or distort clear code to satisfy it.
- Before adding crates, dependencies, public APIs, scripts, checks, abstractions, protocols, tests, or documentation, identify the concrete requirement. Compare removal or reuse with the proposed addition and account for its maintenance cost.
- Get approval before creating a crate, changing a crate's public interface, adding a workspace dependency, or adding an external crate dependency.
- Create crates with `cargo new crates/<name>`. Every crate except the designated unsafe crates inherits the workspace edition and lints.
- After the same correction class occurs twice, use `/correct`. Get approval before building a new prevention check.
- Upgrade toolchain pins together with diagnostic fixes or approved central lint changes. Run `cargo xtask setup` and `cargo xtask check`, then commit the upgrade together.
- Enable the hook once per clone with `git config core.hooksPath .githooks`. `.agents/setup` does this in orbs.

For review requests, inspect the scope the user names. Otherwise review against the merge base with `origin/main`, falling back to `main`, including uncommitted and untracked files. Read whole affected functions, types, and modules where bounds or ownership require context. Run the checks and report failures first without duplicating tool diagnostics. Review prose against the documentation rules below. Report violations, unresolved concerns, and enforcement gaps with file locations, the applicable rule, and a concrete fix. Do not edit code during a review-only request.

## Architecture

- The platform owns execution, external access, and game-memory bytes. It never interprets those bytes or implements game-side mechanisms. The game owns memory layout, initialization, meaning, and game rules. `game-core` owns its reusable mechanisms' invariants.
- `platform-api` contains shared contracts and their validation. It contains no game selection, game mechanisms, or backend implementations. Boundary signatures use its types and an opaque game-owned state root. Selection belongs in `game-entry`.
- Keep platform and third-party library types out of game-side interfaces, including nested types and errors. Keep target-specific code and libraries inside the platform module for that target.
- The platform supplies bounded persistent and transient byte regions. Game-side code constructs arenas over them. Long-lived state stays in the persistent root. Frame scratch resets together.
- Game-side crates are `platform-api`, `game-core`, `game-memory`, `game`, `examples`, and `game-entry`. They use `#![no_std]`, never link `alloc` or `std` through `extern crate`, and have runtime dependencies that use neither. Check transitive dependencies and enabled features when versions or features change. Host-only build dependencies may use both.
- Extract mechanisms into `game-core` only from concrete game use and with game-independent semantics. Retain extracted mechanisms after their original games are removed. No callers in the current `game` crate alone is not grounds for removal.
- Use every added or expanded `game-core` capability in an example in the same change. Demonstrate it through normal execution, not unused imports, no-effect calls, or tests alone. Name the example, runtime path, and observed behavior that depends on the capability. The benchmark need not demonstrate every mechanism.
- Each example owns its logic, assets, and build code in one directory. Never reuse another example's code or assets through helpers, source inclusion, or filesystem paths. Engine utilities, `asset-packer`, and platforms may be shared. Register every example through `game-entry` and check it through both hosts. Do not expose sibling modules to bypass selection or introduce a game interface outside `platform-api::Game`.
- Persistent game memory holds values, indices, handles, and offsets, never references or pointers. Its contents must remain valid when game code reloads.

## Rust and runtime rules

- Use newtypes for confusable IDs, indices, units, and coordinate spaces. Use enums instead of related booleans or `Option` combinations. Keep fields private where constructors enforce validity.
- Express fixed maxima as constants or const generics. Use `debug_assert!` only when removing the check cannot compromise correctness.
- Internal bound violations panic. Return external exhaustion as an error. Never silently exceed a designed bound.
- Do not use runtime recursion, including mutual recursion or recursion through closures or trait objects. Use bounded iteration or an explicit bounded stack. Non-terminating loops do bounded work per iteration.
- Expected failures callers can act on use `Result`. Use `unwrap` or `expect` only for an immediate, locally obvious invariant or an intentionally fatal layer, never for external input or I/O. Discard results deliberately with `let _ =`.
- Express intentional overflow with `checked_`, `wrapping_`, or `saturating_` operations. Arithmetic on external input must return expected errors rather than panic.
- Borrow unless ownership must move. Keep mutable borrows and views short. Use `Rc`, `Arc`, `RefCell`, `Cell`, `Mutex`, and `.clone()` only for their required semantics, not to escape an ownership problem.
- Let callers supply storage for dynamically sized results where practical. Arena and pool interfaces expose ordinary references and slices. Do not combine unrelated lifetimes in one arena.
- Choose AoS, SoA, or a hybrid from actual access patterns. Use a hot/cold split when it reduces the working set. Do not specialize small or cold data without need.
- Keep derived data's source and rebuild or invalidation rule explicit. Separate computation from mutation where it helps reasoning, without forcing purity.
- Prefer synchronous, run-to-completion code. Do not add async runtimes, callback registries, event buses, macro machinery, or trait-object layers merely to reduce line count.
- Model temporal behavior as state machines. Collect external events and process them at defined frame or tick boundaries. Introduce concurrency only for a concrete workload, with ownership, completion, cancellation, and synchronization visible.
- Use fixed-width integers and explicit layout where representation is a contract, including serialization, GPU buffers, FFI, and packed IDs. Use `usize` for lengths and indexing. Make potentially lossy conversions' invariants visible.
- Set dependency options that affect correctness, compatibility, persistence, security, or resources explicitly, including features and `default-features`. Unlike the global default-options rule, do not restate harmless defaults.
- Include stack use and per-frame allocation in cost estimates. Avoid per-frame allocation in hot paths and quadratic work over entity counts.

## Unsafe code

Unsafe code, raw pointers, and ABI details belong only in `UNSAFE_CRATES` in `crates/xtask/src/main.rs`, currently `game-memory`. Never put them in game logic. Adding an unsafe crate needs approval as a central policy change. Otherwise use safe library APIs.

An unsafe crate copies the workspace lint tables, changing only `unsafe_code` to `"allow"`. Update the copy whenever workspace lints change.

Keep unsafe blocks narrow and behind safe interfaces. Each SAFETY contract must prove soundness for every input a safe caller can supply. Safe callers must not need pointer, representation, aliasing, or lifetime knowledge. Miri must exercise every unsafe block.

Do not use unchecked operations such as `get_unchecked` for convenience or performance alone. Establish a necessary operation's invariant first and isolate it behind a safe interface.

## Comments and documentation

The global comment ban also covers trailing comments and `#[doc = "..."]`. Project exceptions are `// SAFETY:` contracts, `# Safety` sections on unsafe functions, and generated-file headers naming the generator and warning against manual edits. Safety text states the proof obligation that makes the operation sound.

Code is the source of truth. Keep prose to non-obvious rationale, setup, policy, legal requirements, and executable verification recipes. Do not add demo-game guides or repeat behavior evident in code. Tool READMEs are standalone usage guides covering invocation, inputs, outputs, and integration. Omit conversation history and unrelated engine design.

## Verification

Unit tests are allowed only for small, reusable utilities in `game-core` and `game-memory`. Apply the global behavior-testing rules through their public interfaces.

Verify game-layer changes only through the production CLI, with controlled input and complete outputs. Run platform-specific verification only when that specific platform is altered. Game-layer changes alone never trigger graphical, keyboard, window, or device-playback checks. Apply the same boundary to performance measurements and verification-skill maintenance. Static checks still cover every configured target.

Follow the repository's `verify-game` skill for commands and evidence. Report unmeasured performance impact as a limitation.
