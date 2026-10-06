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
| `rust-toolchain.toml` | exact Rust toolchain, rustfmt, and Clippy versions; the deployment targets |
| crate dependency graph | layer direction: `game-core` cannot depend on `game` (cycle) or `platform` (binary) |
| `Cargo.toml` `[workspace.lints]` | fatal warnings; no unsafe code outside the unsafe crates; Clippy `all` + `pedantic` (including lossy casts); `dbg!`, `todo!`, `unimplemented!`; outer `#[allow]`; SAFETY contract presence, granularity, and necessity; non-terminating loops must return `!`; visible lifetimes; no unreachable `pub` |
| `clippy.toml` | API-shape lints also apply to public items |
| `Cargo.toml` `[profile.release]` | overflow checks in release builds |
| `scripts/check.sh` | fmt; Clippy for every deployment target; tests on the host, including `platform` only when the host is a deployment target; Miri tests for the unsafe crates on an exactly pinned nightly (`miri_toolchain`); all with `--locked` |
| `.githooks/pre-commit`, `.github/workflows/ci.yml` | running `scripts/check.sh` |
| this file | working rules and design rules, including configuration correctness, comments, and local lint suppressions not caught by static checks |

Configured checks are fatal and have no local escape hatches. If a diagnostic is wrong for this project, change the policy centrally; do not distort clear code to satisfy it. Add individual `nursery` or `restriction` lints only when they give consistent signal, and never enable the whole `restriction` group. When a rule moves to a higher layer, update the table and delete the rule here.

Keep `scripts/check.sh` a runner for standard tools, not a home for custom source checks or duplicated lint policy. Do not add automated checks that validate repository configuration itself. Configuration correctness, including lint inheritance and deployment targets, belongs in review; consuming configuration to run tools does not require a separate validation layer.

Review must reject local lint suppressions not caught by compiler or Clippy policy, including inner `#![allow]`, `#[expect]`, and their `cfg_attr` forms.

## Working rules

- Run `scripts/check.sh` after changing anything. Work is not done until it passes. Never bypass the hook with `--no-verify`.
- When a check fails, fix the code. If the diagnostic is wrong, stop and propose a central change to the user instead of dodging it locally.
- Use `/gamz-review` for every review request in this repository. Before finishing a change to Rust code, manifests, or tooling, run it on the change and resolve its findings.
- Get the user's approval before creating a crate, changing a crate's public interface, adding a dependency between workspace crates, or adding an external crate dependency.
- Create crates with `cargo new crates/<name>`; it inherits the workspace edition and lints. Every crate except the unsafe crates must inherit the workspace lints; check this when reviewing manifest changes.
- Unsafe code, raw pointers, and ABI details live only in the unsafe crates listed in `scripts/check.sh` (today `game-memory`), never in game logic. Adding one is a central policy change that needs the user's approval; otherwise choose libraries with safe APIs. An unsafe crate copies `[workspace.lints]` into its own `[lints]` tables with only `unsafe_code` changed to `"allow"`, and the copy changes whenever the workspace lints do. Miri tests in an unsafe crate exercise every unsafe block.
- Upgrade the toolchain deliberately: change the exact pin (or `miri_toolchain`), run `scripts/check.sh`, fix new diagnostics or reject lints centrally, and commit everything together.
- Enable the hook once per clone with `git config core.hooksPath .githooks`. `.agents/setup` does this in orbs.

## Design rules

### Layers
- Keep the platform a thin host adapter: it owns execution, access to the outside world, and the bytes of game memory, but never interprets those bytes or implements game-side mechanisms. The game owns their layout, initialization, and meaning, and all game rules. `game-core` owns the invariants of the reusable mechanisms it provides.
- `platform-api` is the shared boundary: contracts and their validation only, never game mechanisms or backend implementations. Game/platform boundary signatures use its types; static game entrypoints may additionally expose an opaque game-owned state root.
- Only `game` directly depends on `game-core`. Platform and renderer crates use `platform-api`, never `game-core` or `game-memory`.
- Platform and third-party library types never appear in game-side interfaces; the game expresses intent, not backend mechanism. Target-specific code and platform libraries stay inside the platform module for their target.
- Platform supplies bounded persistent and transient byte regions; game-side code constructs arenas over them. Long-lived state stays inside the persistent root; frame scratch resets together. Game-side crates (`platform-api`, `game-core`, `game-memory`, `game`) are `#![no_std]`, never use `extern crate` to link `alloc` or `std`, and depend only on crates that use neither.
- `game-core` holds only mechanisms whose semantics are independent of this game and already justified by use in `game`.
- Persistent game memory holds values, indices, handles, and offsets, never references or pointers, so its contents stay valid when game code is reloaded.

### Comments
- No comments, including trailing comments and `#[doc = "..."]` attributes. Intent belongs in names, types, assertions, and structure. This rule is enforced in review.
- The only exception is a `// SAFETY:` contract, or a `# Safety` doc section on an `unsafe fn`, that states the proof obligation making the operation sound.

### Documentation
- Code is the source of truth. Do not document demo games or restate what is evident in code; keep documentation to non-obvious rationale, setup, policy, and legal requirements.

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
