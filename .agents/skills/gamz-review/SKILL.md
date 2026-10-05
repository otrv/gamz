---
name: gamz-review
description: "Reviews gamz changes against the priorities, enforcement policy, and design rules in AGENTS.md that compiler, Clippy, and scripts/check.sh cannot enforce. Use for /gamz-review, when asked to review a gamz diff, commit, or PR, and before finishing any change in this repository."
---

# gamz review

Judgment review of a change against [AGENTS.md](../../../AGENTS.md). Static checks own every rule they can express. This review owns the rest and does not duplicate them.

## Workflow

1. Determine the scope: the paths, commits, or PR the user named. Otherwise use the diff against the merge base with `origin/main` (fall back to `main`), including uncommitted and untracked files. Read whole touched functions, types, and modules when a rule needs context, such as bounds or ownership.
2. Run `scripts/check.sh`. If it fails, report that as the first finding and keep reviewing. Do not re-report anything the compiler, Clippy, rustfmt, or the script detects.
3. Check the change against every **Design rules** subsection of `AGENTS.md`, and against **Priorities**.
4. If the change touches an unsafe crate, check that every SAFETY contract proves soundness for all inputs a safe caller can supply, that the safe API cannot be misused to break it, and that its Miri tests exercise every unsafe block.
5. If the change touches `rust-toolchain.toml`, `Cargo.toml` lint tables or profiles, `clippy.toml`, `scripts/check.sh`, hooks, CI, or `AGENTS.md`, also check it against **Enforcement** and **Working rules**. Confirm that the toolchain pin stays exact, that each unsafe crate's `[lints]` tables equal `[workspace.lints]` except `unsafe_code = "allow"`, that lints are not loosened to admit code that should change, and that every rule still has exactly one owner.
6. Report. Do not edit code unless the user asks for fixes.

Rules this review keeps catching that a higher layer could express are tooling gaps. So is anything a static layer should have caught but missed.

## Report

Group findings by kind. For each, cite `file:line`, the rule as `AGENTS.md › <subsection>`, why it applies, and a concrete fix:

- **Violation**: breaks a rule.
- **Concern**: a judgment call worth discussing.
- **Tooling gap**: something a higher enforcement layer could own.

If nothing applies, say "No findings." Do not praise or restate static-check results.
