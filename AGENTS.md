# gamz

Rust game engine boilerplate. Every workspace crate lives in `crates/<name>`.

Inherit the user's global `AGENTS.md` without copying it here.

## Load guidance by task

- **Implement:** Read the [design priorities](CODING_STANDARDS.md#design-constraints) and [architecture boundaries](CODING_STANDARDS.md#architecture-and-memory). Before changing storage, representation, runtime behavior, unsafe code, tests, or prose, read the matching section. Consult [review criteria](CODING_STANDARDS.md#review-criteria) for the API and style choices in scope.
- **Choose an owner:** Before adding or moving a responsibility, consult [layer ownership](CODING_STANDARDS.md#layer-ownership). Identify its owning crate, required shared guarantees, and implementation choices that remain local.
- **Prose:** Load `writing-for-agents` before drafting or revising any prose, including documentation, commit messages, and responses. Check the final text against the skill before delivering it. Preserve the intended audience. Keep README files human-facing and agent workflow guidance in agent instructions.
- **Review:** Read all of `CODING_STANDARDS.md`. Apply every relevant rule, including constraints and enforcement-maintenance rules. Review evidence independently of the implementer's conclusions.
- **Change enforcement:** Before changing crates, dependencies, toolchains, lint configuration, setup, builds, or checks, read [enforcement maintenance](CODING_STANDARDS.md#enforcement-maintenance).

The compiler, Clippy, rustfmt, and cargo-deny own their configured checks. Prose covers decisions and gaps those tools cannot enforce. Implementers and reviewers use the same rules, not separate copies.

## Policy changes require explicit approval

Get the user's explicit approval for the exact rule and proposed relaxation before broadening any policy or weakening its enforcement. This includes architecture, safety, tests, dependencies, lint settings, checks, and agent instructions. Approval to implement a feature or refactor does not approve policy exceptions.

When implementation conflicts with a policy, preserve the policy and explain the conflict. Propose a compliant design or ask for the specific exception before proceeding. Apply this gate to configuration and code as well as prose; moving or renaming a violation does not resolve it.

## Implement and finish

1. Identify the concrete requirement before adding crates, dependencies, public APIs, scripts, checks, abstractions, protocols, tests, or documentation. Compare reuse or removal with the addition and account for maintenance cost. Use `/grill-me` for unresolved design decisions.
   For cross-layer changes, agree on a short interface sketch in the conversation before implementation or delegation. Name each responsibility's owner and what remains deferred.
2. Get approval before creating a crate, changing a crate's public interface, adding a workspace dependency, or adding an external crate dependency. Create approved crates with `cargo new crates/<name>`.
3. Finish the implementation, integration, and cleanup. Resolve concrete correctness and design problems. Optional optimizations and hypothetical requirements are not technical debt.
4. Verify the completed changes using the scope below, then run `.githooks/pre-commit`. Use `.githooks/pre-commit --all` for full validation. Install missing prerequisites with `./scripts/setup`. Never use `--no-verify`. If a diagnostic is wrong, stop and propose a central policy change instead of suppressing it or distorting clear code.
5. Fix every substantiated finding. Resolve disputed findings with evidence and escalate unresolved policy decisions to the user. After fixes, repeat affected runtime verification and the hook. Finish only when the final tree has passing checks and no unresolved findings.

Give parallel workers separate file ownership and targeted checks. The integrating agent runs the required hook after edits have settled, rather than each worker running it against concurrent changes.

After the same correction class occurs twice, use `/correct`. Get approval before adding a prevention check.

## Verify behavior and evidence

Use commands that propagate failures, including failures inside pipelines. Record decisive output and distinguish passed, failed, canceled, and unrun checks. Passing static checks does not prove runtime behavior.

Before removing a temporary probe used as evidence, preserve its fixture, assertions, commands, complete output, and tested source identity together in review artifacts. Include uncommitted source changes in that identity. These artifacts are not permanent repository tests.

Verify game behavior only through the production CLI, with controlled inputs and complete outputs. Follow `verify-game` for commands and evidence. Derive expected results independently and assert the changed behavior. Protocol success and matching replay alone do not establish gameplay correctness.

Select runtime hosts by the changed contract:

- Game-only changes use the CLI.
- Host or renderer changes require verification of the affected hosts.
- Game-host API changes require re-verification of each host that consumes the changed contract.

Before a platform run, name the changed host, renderer, or API code and the affected property. Game-only changes and verification-skill maintenance do not justify graphical, keyboard, window, or device-playback checks. Apply this scope to performance measurements too. Static compilation coverage remains the hook's responsibility and does not require running every host.

Keep `verify-game` and its feature map game-independent. Engine-contract assertions accept game-independent outputs. Supply game-specific workloads and assertions as task-local files. Keep the helper CLI-only.

Report unmeasured performance and unverified hardware behavior as limitations. Software rendering and null-audio runs do not establish physical GPU performance or audible playback.

## Review independently

Start an oracle review or reassessment only when the user explicitly requests it. Implementation and verification alone do not authorize a review. For a requested oracle review, supply the original requirements, approvals, exact review scope, changed paths, verification commands and results, and limitations. Ask oracle to follow the procedure below without assuming the implementation is correct.

1. State the scope. Use the user's named scope. Otherwise compare against the merge base with `origin/main`, falling back to `main`, including uncommitted and untracked files. Report an empty scope as empty, not as a repository-wide clean review.
2. Read whole affected functions, types, and modules where contracts, bounds, or ownership require context. Apply the standards to code and prose. Check necessity, approval evidence, and enforcement gaps, not just the diff's syntax.
3. Run the checks unless the implementer supplied successful results for the exact reviewed tree. Inspect those results and state which checks you ran or relied on. Independently probe missing or questionable evidence within the verification scope above. Never infer a pass from an intended command or a screenshot alone.
4. Report failed checks first without duplicating diagnostics. Give each finding a location, violated contract or rule, consequence, and concrete fix. Separate defects from optional improvements. State unverified behavior even when there are no findings.

Review-only work is read-only. Oracle reports findings to the implementer rather than editing files or starting another review cycle itself.

## Workflow enforcement

| Rule | Enforcement |
| --- | --- |
| [Policy approval gate](#policy-changes-require-explicit-approval) | Agent must identify explicit user approval for the exact relaxation before editing. Existing compiler and hook checks remain enabled. Repository checks cannot determine whether a prose change weakens a policy or whether the user authorized it. |
| [Oracle request gate](#review-independently) | Agent checks the user's request before invoking oracle. Repository checks cannot observe agent tool calls. |
| [Prose guidance](#load-guidance-by-task) | Agent loads the skill and checks the final text. Repository checks cannot verify skill use or judge the intended audience. |
| [Layer ownership](CODING_STANDARDS.md#layer-ownership) | Cargo-deny restricts consumers of `game`, `game-core`, `memory`, and `renderer-wgpu`; the hook and CI run it. Review checks responsibility placement. Dependency checks cannot determine whether implementation logic belongs in a contract crate. |
| [Platform independence](CODING_STANDARDS.md#layer-ownership) | The `Platform` function table borrows host-owned context. Linux file, window, and texture state is crate-private. The old `Services`, platform trait, and memory-registration APIs are removed. Rust rejects callers of those removed APIs. Review checks ownership; no automated check prevents adding a new shared implementation layer. |
| [Declaration-only boundary](CODING_STANDARDS.md#layer-ownership) | Plain fields use shared math types. Removed API limits, numeric wrappers, and constructors cannot be imported. Renderer capacities must be supplied by its caller. Review checks that new boundary types remain declarations and that limits stay platform-owned; existing tools do not forbid adding new logic to this crate. |
| [Lifecycle and borrowed data](CODING_STANDARDS.md#layer-ownership) | `Game` separates frame and audio updates. `Frame` has no uploads. The callback table has no game memory setter. Borrowed completion events cannot outlive their batch, and returned frames cannot borrow input events or callback context. Rust enforces these signatures and lifetimes; runtime checks must establish host scheduling, copying, and completion behavior. |
| [Utility ownership and composition](CODING_STANDARDS.md#architecture-and-memory) | Rust privacy prevents access to font representation outside its owner. Public API tests exercise font output and command storage independently. Review checks sibling dependencies and caller composition; no automated check currently forbids all sibling imports. |
| [Numeric and capacity contracts](CODING_STANDARDS.md#tests) | Public utility tests cover finite interpolation endpoints, extreme magnitudes, emitted glyph counts, and exact or exhausted output budgets. The existing hook runs these tests locally and in CI. |
