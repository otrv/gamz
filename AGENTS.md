# gamz

Rust game engine. Cargo workspace; every crate lives in `crates/<name>`.

## Priorities

Correctness, then predictability, then performance, then simplicity. Never trade a higher priority for a lower one.

Choose the simplest design that preserves the known invariants. Do not add machinery for hypothetical problems, and do not postpone a known correctness or design violation in the name of simplicity. The goal is ordinary, idiomatic Rust with unusually strong guardrails, not custom machinery.

## Enforcement

Each rule is owned by the highest layer that can express it, and lower layers do not restate it:

1. types, visibility, and compile-time structure;
2. compiler and Clippy policy;
3. rustfmt;
4. `scripts/check.sh`, run by the pre-commit hook and CI;
5. this file, checked by the `/gamz-review` skill;
6. a new tool, only when it prevents a meaningful class of defects the toolchain cannot express and clearly outweighs its maintenance cost.

| Layer | Owns |
|---|---|
| `rust-toolchain.toml` | exact Rust toolchain, rustfmt, and Clippy versions |
| `Cargo.toml` `[workspace.lints]` | fatal warnings; no unsafe code outside the unsafe crates; Clippy `all` + `pedantic` (including lossy casts); `dbg!`, `todo!`, `unimplemented!`; outer `#[allow]`; SAFETY contract presence, granularity, and necessity; non-terminating loops must return `!`; visible lifetimes; no unreachable `pub` |
| `clippy.toml` | API-shape lints also apply to public items |
| `Cargo.toml` `[profile.release]` | overflow checks in release builds |
| `scripts/check.sh` | no comments starting a line except SAFETY contracts; no inner `#![allow]`; no `#[expect]` in any form; every crate except the unsafe crates (`unsafe_crates`) inherits the workspace lints; fmt, check, Clippy, and tests; Miri tests for the unsafe crates on an exactly pinned nightly (`miri_toolchain`); all with `--locked` |
| `.githooks/pre-commit`, `.github/workflows/ci.yml` | running `scripts/check.sh` |
| this file | working rules and design rules |

Configured checks are fatal and have no local escape hatches. If a diagnostic is wrong for this project, change the policy centrally; do not distort clear code to satisfy it. Add individual `nursery` or `restriction` lints only when they give consistent signal, and never enable the whole `restriction` group. When a rule moves to a higher layer, update the table and delete the rule here.

## Working rules

- Run `scripts/check.sh` after changing anything. Work is not done until it passes. Never bypass the hook with `--no-verify`.
- When a check fails, fix the code. If the diagnostic is wrong, stop and propose a central change to the user instead of dodging it locally.
- Use `/gamz-review` for every review request in this repository. Before finishing a change to Rust code, manifests, or tooling, run it on the change and resolve its findings.
- Create crates with `cargo new crates/<name>`; it inherits the workspace edition and lints.
- Unsafe code, raw pointers, and ABI details live only in the unsafe crates listed in `scripts/check.sh` (today `game-memory`), never in game logic. Adding one is a central policy change that needs the user's approval; otherwise choose libraries with safe APIs. An unsafe crate copies `[workspace.lints]` into its own `[lints]` tables with only `unsafe_code` changed to `"allow"`, and the copy changes whenever the workspace lints do. Miri tests in an unsafe crate exercise every unsafe block.
- Upgrade the toolchain deliberately: change the exact pin (or `miri_toolchain`), run `scripts/check.sh`, fix new diagnostics or reject lints centrally, and commit everything together.
- Enable the hook once per clone with `git config core.hooksPath .githooks`. `.agents/setup` does this in orbs.

## Design rules

### Comments
- No comments. Intent belongs in names, types, assertions, and structure. `scripts/check.sh` sees only comments that start a line; trailing comments and `#[doc = "..."]` attributes are equally forbidden.
- The only exception is a `// SAFETY:` contract, or a `# Safety` doc section on an `unsafe fn`, that states the proof obligation making the operation sound.

### Invariants
- Make invalid states unrepresentable: newtypes for confusable values such as IDs, indices, units, and coordinate spaces; enums instead of related booleans or `Option` combinations; private fields and constructors that reject invalid states.
- Express genuinely fixed maxima as constants or const generics.
- Assert invariants the types cannot express. Use `debug_assert!` only where removing the check cannot compromise correctness.

### Bounds
- Give everything that can grow, repeat, queue, retry, or consume resources a deliberate upper bound and an explicit failure policy: collections, buffers, channels, retries, loops over external input, and allocations sized by input. Prefer statically visible bounds; otherwise establish the bound at a trusted boundary and preserve it.
- An internal bound violation is a programmer error and panics; external exhaustion is returned as an error. Never silently grow past a designed bound.
- No runtime recursion, including mutual recursion and recursion through closures or trait objects. Use an explicit bounded stack or iteration.
- Loops that never terminate do bounded work per iteration.

### Failures
- Broken invariants panic. Expected failures that callers can act on are `Result`s. Do not build recovery paths for programmer errors, and do not terminate on expected failures to shorten control flow.
- `unwrap` and `expect` are fine when the invariant is immediate and locally obvious, or when the layer is intentionally fatal. Never use them on external input or I/O.
- Discard results only deliberately, with `let _ =`.

### Unsafe and unchecked operations
- In designated low-level crates, keep each unsafe block narrow and behind a safe interface. Safe callers must not need to know the pointer, representation, aliasing, or lifetime rules.
- Do not use unchecked operations such as `get_unchecked` for performance or convenience. When one is necessary, establish its invariant first and isolate it the same way as other unsafe code.

### Arithmetic
- Express intentional overflow with `checked_`, `wrapping_`, or `saturating_` operations. Arithmetic on external input must not panic where an error is expected.

### Ownership and lifetimes
- Move ownership only when needed; otherwise borrow, preferring slices and other bounded views. Keep mutable borrows and borrowed views short.
- Use `Rc`, `Arc`, `RefCell`, `Cell`, `Mutex`, and `.clone()` only when their semantics are required, never to escape a difficult ownership model.
- Solve the lifetime problem first, then choose the allocation mechanism. Values that die together, such as per-frame or scratch data, are released or reset together. Do not create an arena without a shared end point, and do not merge unrelated lifetimes for convenience.
- Let callers choose the storage for dynamically sized results where practical, such as `&mut Vec<T>` or an output slice. Arena-backed or pooled storage still exposes ordinary references and slices.

### Data layout
- For data processed in bulk, use contiguous storage, handles or indices instead of pointer graphs, and a hot/cold split where it reduces the working set.
- Choose AoS, SoA, or a hybrid from the actual access pattern. Do not contort small or cold data into a specialized layout.

### State
- Keep one authoritative representation per fact. Derived or cached data needs a clear source and rebuild or invalidation rule.
- Separate computation from mutation and effects where that simplifies reasoning, without forcing purity. Place behavior where its invariants are easiest to understand, not necessarily on the type it transforms.

### Control flow
- Prefer direct, synchronous, run-to-completion code. No async runtimes, callback registries, event buses, macro machinery, or trait-object layers whose main benefit is fewer lines.
- Model temporal behavior as explicit state machines, not flags or call-order dependencies. Keep decisions near the state they change.
- Collect external events and process them at defined frame or tick boundaries.
- Introduce concurrency only for a concrete workload, with the narrowest mechanism, keeping shared ownership, pending work, completion, cancellation, and synchronization visible.

### Boundaries
- A module owns a coherent set of invariants and hides decisions likely to change. Callers cannot bypass those invariants through public fields, shared mutable state, or escape hatches.
- Interfaces use domain types and enums instead of primitives and booleans, with names that distinguish materially different concepts.
- Introduce an abstraction only when it hides a meaningful decision, enforces an invariant, or compresses understood behavior. No forwarding layers, speculative generics or traits, or mechanical mirroring of domain nouns.
- Prefer composition and explicit data flow over trait hierarchies unless substitutability is a real domain property.

### Representation
- Use fixed-width integers and explicit layout (`#[repr]`, byte order) where width or layout is a contract: serialization, file formats, GPU buffers, FFI, and packed IDs. Use `usize` for lengths and indexing, and natural types for other in-memory values.
- Make the invariant behind any potentially lossy conversion visible.

### Dependencies
- Add a dependency when it removes substantial work or defect risk, not for trivial convenience. Keep its surface narrow, and avoid dependencies that impose a runtime, scheduler, allocator, ownership model, or framework lifecycle on unrelated code.
- Set dependency options that affect correctness, compatibility, persistence, security, or resources explicitly, such as `default-features` and features. Do not restate harmless defaults.

### Budgets
- Before committing to a design with significant cost, estimate bytes per item, item counts, per-frame work, buffer sizes, I/O, and stack use.
- Avoid obviously wasteful choices, such as per-frame allocation in hot paths or quadratic work over entity counts. Batch along natural units of work. Back additional optimization complexity with measurements.
