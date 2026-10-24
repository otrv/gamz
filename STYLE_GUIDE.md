# Rust style guide

This document defines the project's code style, architecture, memory model,
runtime rules, and documentation policy. Rank correctness, predictability, and
performance in that order. Choose the simplest design that preserves those
priorities. Prefer readable, maintainable code over convenience when writing it.

The rules apply to handwritten and generated code, including macros, generic
signatures, nested types, and errors. Style choices do not authorize exceptions
to memory or architecture rules, public-interface changes, or new dependencies.
Memory, architecture, and safety constraints take precedence over style
preferences. Do not rename APIs or reorganize crates solely for a preference.
Configured diagnostics remain fatal. Do not distort clear code to satisfy a
diagnostic or suppress it locally. Propose a central policy change instead.

## Architecture and memory

The platform owns execution, external access, and game-memory bytes. It never
interprets those bytes or implements game-side mechanisms. The game owns memory
layout, initialization, meaning, and game rules. `game-core` owns its reusable
mechanisms' invariants.

Keep dependencies directed from consumers to the mechanisms they use.
`game-core` cannot depend on `game` or the `platform` binary.

`platform-api` contains shared contracts and their validation. It contains no
game selection, game mechanisms, or backend implementations. Boundary signatures
use its types and an opaque game-owned state root. Hosts link `game` directly.
Keep platform and third-party library types out of game-side interfaces,
including nested types and errors. Keep target-specific code and libraries
inside the platform module for that target.

The platform supplies bounded persistent and transient byte regions. Game-side
code constructs arenas over them. Long-lived state stays in the persistent root.
Frame scratch resets together. Persistent game memory holds values, indices,
handles, and offsets, never references or pointers. Its contents must remain
valid when game code reloads.

Game-side crates are `platform-api`, `game-core`, `game-memory`, and `game`.
They use `#![no_std]`, never link `alloc` or `std` through `extern crate`, and have
runtime dependencies that use neither. Check transitive dependencies and enabled
features when versions or features change.
Host-only build dependencies may use both. Host allocation does not authorize
game-side heap allocation or per-frame allocation in hot paths.

Extract mechanisms into `game-core` only from concrete game use and with
game-independent semantics. Retain extracted mechanisms after their original
games are removed. No callers in the current `game` crate alone is not grounds
for removal.

Use every added or expanded `game-core` capability in `game` in the same
change. Demonstrate it through normal execution, not unused imports, no-effect
calls, or tests alone. Name the runtime path and observed behavior that depends
on the capability.

Keep game logic, assets, and build code in `crates/game`. Check the game through
every host. Do not introduce a game interface outside `platform-api::Game`.

## Code organization and imports

Keep the Cargo layout under `crates/<name>`. Name a leaf module after its primary
type when it has one. Prefer module declarations and re-exports in non-leaf
modules, with a small amount of code that connects their parts when needed.
Neither preference requires a new crate or a public re-export.

Avoid glob imports and new preludes. Glob imports are permitted for `super::*`
in inline modules, enum variants in functions centered on that enum, and
submodule re-exports in non-leaf modules. These exceptions do not authorize new
public interfaces or bypass visibility and game-selection rules. A third-party
prelude requires a concrete benefit accepted in review.

Import items from one module per `use` statement. Do not nest paths inside an
import group. Prefer function-local trait imports when their methods are used
only there. Import types and traits by name, unless qualification or an alias
avoids ambiguity or makes a large family of related types clearer. Macros and
generated code may use qualified paths where imports are impractical.

Qualify free functions from other modules with their module name. Direct imports
are acceptable when the function name already identifies its purpose clearly or
the module name would create ambiguity.

Add APIs and abstractions only for concrete requirements. Compare reuse or
removal with an addition and account for its maintenance cost. Keep application
code direct rather than generalizing it for hypothetical callers.

Set dependency options that affect correctness, compatibility, persistence,
security, or resources explicitly, including features and `default-features`.
Do not restate harmless defaults.

## Formatting and names

Use the pinned rustfmt through `cargo fmt --all`. Keep the default 100-column
width. Prefer lowercase hexadecimal digits and escapes for invisible characters
in literals. Unavailable stable rustfmt options do not justify a nightly
formatter or extra enforcement scripts.

Use `#[rustfmt::skip]` only for meaningful manual alignment or generated code,
never for personal formatting preferences. A formatting skip is not a lint
suppression and does not exempt the code from other checks.

Use `UpperCamelCase` for types and traits, `snake_case` for modules, functions,
and variables, and `SCREAMING_SNAKE_CASE` for constants and statics. For conversion
methods, use `as_` for a borrowed view, `to_` for a conversion that borrows the
source, and `into_` for a conversion that consumes the source.

Avoid unfamiliar abbreviations and words shortened by removing letters.
Use descriptive type lifetimes, except in very generic types.
Short lifetimes are acceptable for simple local input-output relationships and
higher-ranked bounds. Use descriptive names when more than two local lifetimes
are needed. Write explicit lifetimes in type paths. Use `'_` where the lifetime
can be inferred without naming a relationship.

Name generic parameters for what they represent. `T` and `U` are appropriate for
unconstrained types. Name extension traits with an `Ext` suffix.

Use `new` for constructors and `new_with_foo` for alternatives. Use `try_new` for
constructors returning `Result`. Use `from_foo` for value conversions with a
matching `From` implementation when that trait's contract fits. Naming rules do
not require duplicate constructors or authorize public API changes.

## Ownership and storage

Borrow unless ownership must move. Keep mutable borrows and views short. Use
`Rc`, `Arc`, `RefCell`, `Cell`, `Mutex`, and `.clone()` only for their required
semantics, not to escape an ownership problem. None of these types permits
allocation where the memory rules prohibit it.

Borrow data directly rather than borrowing its container. Prefer `&T`, `&[T]`,
`&str`, and `Option<&T>` when the wrapper itself is not part of the contract.
In host code, prefer these to `&Box<T>`, `&Vec<T>`, `&String`, and `&Option<T>`
when the wrapper itself is not needed.

Let callers supply storage for dynamically sized results where practical. Arena
and pool interfaces expose ordinary references and slices. Do not combine
unrelated lifetimes in one arena.

Prefer iterators over newly allocated return collections when callers can
consume results incrementally. In host code, returning an owned collection is
appropriate when the implementation already owns one, callers need indexing or
ownership, or an iterator adds complexity without a useful saving.

For reusable APIs that only iterate, consider `IntoIterator` when callers benefit
from different input sources. Keep a slice when it expresses the storage or
bounded-work contract more clearly. Do not generalize an API for hypothetical
callers. Use `impl Into<T>` only for a concrete library ergonomics need that
justifies generic code size and inference costs. Prefer concrete application
parameters.

Choose AoS, SoA, or a hybrid from actual access patterns. Use a hot/cold split
when it reduces the working set. Do not specialize small or cold data without
need. Include stack use and per-frame allocation in cost estimates. Avoid
per-frame allocation in hot paths and quadratic work over entity counts.

## Types and representation

Use newtypes for confusable IDs, indices, units, and coordinate spaces. Use enums
instead of related booleans or `Option` combinations. Keep fields private where
constructors enforce validity.

Use named fields rather than multi-field tuple structs or tuples whose positions
are unclear. Newtypes and small, unambiguous local tuples are exceptions. Use an
enum for an unclear boolean parameter rather than adding an argument comment.

Use fixed-width integers and explicit layout where representation is a contract,
including serialization, GPU buffers, FFI, and packed IDs. Use `usize` for lengths
and indexing. Make potentially lossy conversions' invariants visible.

Keep derived data's source and rebuild or invalidation rule explicit. Separate
computation from mutation where it helps reasoning, without forcing purity.

## Traits and related APIs

Use an existing standard trait when its semantics match the operation. In
game-side crates, use traits available through `core`, not traits requiring
`alloc` or `std`. Add implementations for concrete needs, not anticipated users.
Derive implementations when the derived bounds and semantics match the contract.
Keep a manual implementation when deriving would add unwanted bounds or change
behavior.

Use inherent methods unless a trait supports generic programming, an opaque or
dynamic trait interface, or a justified extension trait. Prefer generics to
dynamic dispatch in reusable code, while accounting for code size. Do not add a
trait-object layer merely to reduce line count. Normally, the owner of a foreign
type also owns its extension traits. An exception needs substantial value from
a shared library, not local convenience.

Use `Copy` only when implicit copying fits the semantics and can remain part of
the API. Small value errors are candidates. Use `Clone` only when explicit
duplication is needed. Implement `Default` when a valid default exists, including
an options struct intended for struct-update construction. Never invent a
default that bypasses validation.

Use `PartialEq` for equality and `PartialOrd` for partial ordering. Add `Eq` only
when equality is reflexive, and `Ord` only for a total order. Map keys require
consistent `Hash` and `Eq` implementations. Public fields alone do not justify
copying every trait implemented by their fields.

Pair `len` with `is_empty`. Use operator overloads for their mathematical or
logical purpose. Add `#[must_use]` when ignoring a return value prevents correct
use, including pure results. Prefer a type-level annotation for error-like or
lazy result types where appropriate. Avoid redundant function annotations when
the return type already provides the warning.

Treat another owner's `#[doc(hidden)]` items as private. Make public functions
`const` only when that guarantee can be maintained or callers can be migrated.
Use `#[non_exhaustive]` for evolving APIs consumed outside coordinated ownership.
It is not required for every public workspace type, closed domains, APIs whose
callers can be migrated together, or structs intended for external struct-update
construction. Changes to public interfaces still require approval.

Name a getter `foo` and a mutable-reference getter `foo_mut`. Use `take_foo` to
return a value and leave an empty or default state. Use `set_foo` to replace a
value and `replace_foo` to also return the previous value.

Use `with_foo` and `with_mut_foo` for closure accessors when scoped access or
follow-up validation requires them. Do not introduce interior mutability merely
to use that pattern. Arena and pool interfaces expose ordinary references and
slices.

## Errors and bounds

Express fixed maxima as constants or const generics. Use `debug_assert!` only
when removing the check cannot compromise correctness. Internal bound violations
panic. Return external exhaustion as an error. Never silently exceed a designed
bound.

Expected failures callers can act on use `Result`. Ordinary absence can use
`Option`. Use `unwrap` or `expect` only for an immediate, locally obvious
invariant or an intentionally fatal layer, never for external input or I/O.
Discard results deliberately with `let _ =`.

Prefer pattern matching to checking an `Option` and then unwrapping it. When a
panic is permitted, prefer `expect` when its message explains the invariant.
Add assertion context when it explains a precondition, identifies an
implementation bug, or reports useful values. Do not repeat the assertion in
words.

Express intentional overflow with `checked_`, `wrapping_`, or `saturating_`
operations. Arithmetic on external input must return expected errors rather
than panic.

## Execution and concurrency

Do not use runtime recursion, including mutual recursion or recursion through
closures or trait objects. Use bounded iteration or an explicit bounded stack.
Non-terminating loops do bounded work per iteration.

Prefer synchronous, run-to-completion code. Do not add async runtimes, callback
registries, event buses, macro machinery, or trait-object layers merely to reduce
line count.

Model temporal behavior as state machines. Collect external events and process
them at defined frame or tick boundaries. Introduce concurrency only for a
concrete workload, with ownership, completion, cancellation, and synchronization
visible. Do not recover access to the contents of a poisoned mutex.

Do not call `catch_unwind` except as last-resort containment with unwinding
enabled, in code that already prevents avoidable panics. Never rely on that
containment for correctness or expected error handling. Do not run code through
static constructors before `main`. Use explicit platform initialization.

## Implementation choices

Add explicit variable types when generic conversions or complex iterator chains
obscure the result. Prefer a variable annotation to a turbofish when either
works. Use direct loops for complex control flow or mutation. Name a composed
iterator before using it in a `for` header. Prefer a loop over `for_each` unless
the combinator has a clear benefit. Keep short combinators such as `any` when
they express the operation directly.

When matching an external non-exhaustive enum, list known variants relevant to
the operation explicitly. Use `if let` or `let ... else` when only a particular
variant matters, unless `match` is clearer. A wildcard must not silently hide a
known case that needs handling.

Prefer ordinary Rust over macros and code generation. When generation is
necessary, consider `macro_rules!` before external generation and procedural
macros. Existing required derives remain appropriate. Do not add macro machinery
solely to shorten code.

Use stable language and library features. The pinned nightly Miri toolchain is
verification tooling, not permission to introduce unstable production features.

Prefer format strings for substitutions and concatenation of more than two
strings. Use `format!` only where allocation is permitted. Use `core::fmt::Write`
with caller-supplied bounded storage for game-side text when needed, and handle
capacity errors. Formatting does not justify a dependency, a new allocation, or
a cross-boundary error type.

## Unsafe code

Unsafe code, raw pointers, and ABI details belong only in `game-memory`.
Never put them in game logic. Adding an unsafe crate needs approval as a central
policy change and Miri coverage in `.githooks/pre-commit`. Otherwise use safe
library APIs.

Keep unsafe blocks narrow and behind safe interfaces. Each SAFETY contract must
prove soundness for every input a safe caller can supply. Safe callers must not
need pointer, representation, aliasing, or lifetime knowledge. Miri must exercise
every unsafe block.

Do not use unchecked operations such as `get_unchecked` for convenience or
performance alone. Establish a necessary operation's invariant first and isolate
it behind a safe interface.

## Comments and documentation

Do not write comments, including documentation comments, trailing comments, and
`#[doc = "..."]`. Exceptions are `// SAFETY:` contracts, `# Safety` sections on
unsafe functions, and generated-file headers naming the generator and warning
against manual edits. Safety text states the proof obligation that makes the
operation sound.

Code is the source of truth. Keep prose to non-obvious rationale, setup, policy,
legal requirements, and executable verification recipes. Do not add demo-game
guides or repeat behavior evident in code. Tool READMEs are standalone usage
guides covering invocation, inputs, outputs, and integration. Omit conversation
history and unrelated engine design.

Reject handwritten local lint suppressions, including allowances for centrally
allowed lints and attributes in unchecked configurations. The ban also covers
`non_camel_case_types`, `non_upper_case_globals`, `unused_attributes`, and
`clippy::useless_attribute`, despite their central `deny` level. Third-party
macro-generated allowances are not handwritten suppressions.

## Tests

Unit tests are allowed only for small, reusable utilities in `game-core` and
`game-memory`. Test behavior callers rely on through public interfaces, not
private helpers or incidental implementation details. Prefer extending existing
tests over adding new ones. Keep a regression test only when it would catch a
plausible future mistake that types and existing tests would miss.

Choose inputs that distinguish correct behavior from likely mistakes. Derive
expected results independently of the implementation. Verify game-layer behavior
through the production CLI with controlled inputs and complete outputs, not by
adding game unit tests or a foreign test framework.
