# gamz

Rust game engine boilerplate. Every workspace crate lives in `crates/<name>`.

Inherit the user's global `AGENTS.md` without copying it here.

## Load guidance by task

- **Implement:** Read the [design priorities](CODING_STANDARDS.md#design-constraints) and [architecture boundaries](CODING_STANDARDS.md#architecture-and-memory). Before changing storage, representation, runtime behavior, unsafe code, tests, or prose, read the matching section. Consult [review criteria](CODING_STANDARDS.md#review-criteria) for the API and style choices in scope.
- **Review:** Read all of `CODING_STANDARDS.md`. Apply every relevant rule, including constraints and enforcement-maintenance rules. Review evidence independently of the implementer's conclusions.
- **Change enforcement:** Before changing crates, dependencies, toolchains, lint configuration, setup, builds, or checks, read [enforcement maintenance](CODING_STANDARDS.md#enforcement-maintenance).

The compiler, Clippy, rustfmt, and cargo-deny own their configured checks. Prose covers decisions and gaps those tools cannot enforce. Implementers and reviewers use the same rules, not separate copies.

## Implement and finish

1. Identify the concrete requirement before adding crates, dependencies, public APIs, scripts, checks, abstractions, protocols, tests, or documentation. Compare reuse or removal with the addition and account for maintenance cost. Use `/grill-me` for unresolved design decisions.
2. Get approval before creating a crate, changing a crate's public interface, adding a workspace dependency, or adding an external crate dependency. Create approved crates with `cargo new crates/<name>`.
3. Finish the implementation, integration, and cleanup. Resolve concrete correctness and design problems. Optional optimizations and hypothetical requirements are not technical debt.
4. Verify the completed changes using the scope below, then run `.githooks/pre-commit`. Use `.githooks/pre-commit --all` for full validation. Install missing prerequisites with `./scripts/setup`. Never use `--no-verify`. If a diagnostic is wrong, stop and propose a central policy change instead of suppressing it or distorting clear code.
5. Ask an oracle reviewing subagent to inspect the completed changes. Supply the original requirements, approvals, exact review scope, changed paths, verification commands and results, and limitations. Ask it to follow the review procedure below without assuming the implementation is correct.
6. Fix every substantiated finding. Resolve disputed findings with evidence and ask oracle to reassess them. Escalate unresolved policy decisions to the user. After fixes, repeat affected runtime verification and the hook, then return the revised scope to oracle. Finish only when the final tree has passing checks and no unresolved review findings.

After the same correction class occurs twice, use `/correct`. Get approval before adding a prevention check.

## Verify behavior and evidence

Use commands that propagate failures, including failures inside pipelines. Record decisive output and distinguish passed, failed, canceled, and unrun checks. Passing static checks does not prove runtime behavior.

Verify game behavior only through the production CLI, with controlled inputs and complete outputs. Follow `verify-game` for commands and evidence. Derive expected results independently and assert the changed behavior. Protocol success and matching replay alone do not establish gameplay correctness.

Select runtime hosts by the changed contract:

- Game-only changes use the CLI.
- Host or renderer changes require verification of the affected hosts.
- Game-host API changes require re-verification of each host that consumes the changed contract.

Before a platform run, name the changed host, renderer, or API code and the affected property. Game-only changes and verification-skill maintenance do not justify graphical, keyboard, window, or device-playback checks. Apply this scope to performance measurements too. Static compilation coverage remains the hook's responsibility and does not require running every host.

Keep `verify-game` and its feature map game-independent. Engine-contract assertions accept game-independent outputs. Supply game-specific workloads and assertions as task-local files. Keep the helper CLI-only.

Report unmeasured performance and unverified hardware behavior as limitations. Software rendering and null-audio runs do not establish physical GPU performance or audible playback.

## Review independently

1. State the scope. Use the user's named scope. Otherwise compare against the merge base with `origin/main`, falling back to `main`, including uncommitted and untracked files. Report an empty scope as empty, not as a repository-wide clean review.
2. Read whole affected functions, types, and modules where contracts, bounds, or ownership require context. Apply the standards to code and prose. Check necessity, approval evidence, and enforcement gaps, not just the diff's syntax.
3. Run the checks unless the implementer supplied successful results for the exact reviewed tree. Inspect those results and state which checks you ran or relied on. Independently probe missing or questionable evidence within the verification scope above. Never infer a pass from an intended command or a screenshot alone.
4. Report failed checks first without duplicating diagnostics. Give each finding a location, violated contract or rule, consequence, and concrete fix. Separate defects from optional improvements. State unverified behavior even when there are no findings.

Review-only work is read-only. Oracle reports findings to the implementer rather than editing files or starting another review cycle itself.
