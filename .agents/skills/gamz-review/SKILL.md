---
name: gamz-review
description: "Reviews gamz changes against the priorities, enforcement policy, and design rules in AGENTS.md that compiler, Clippy, and scripts/check.sh cannot enforce. Use for /gamz-review, when asked to review a gamz diff, commit, or PR, and before finishing any change in this repository."
---

# gamz review

Judgment review of a change against [AGENTS.md](../../../AGENTS.md). Static checks own every rule they can express. This review owns the rest and does not duplicate them.

## Workflow

1. Determine the scope: the paths, commits, or PR the user named. Otherwise use the diff against the merge base with `origin/main` (fall back to `main`), including uncommitted and untracked files. Read whole touched functions, types, and modules when a rule needs context, such as bounds or ownership.
2. Run `scripts/check.sh`. If it fails, report that as the first finding and keep reviewing. Do not re-report anything the compiler, Clippy, rustfmt, or the script detects.
3. Check every change against **Priorities**, **Enforcement**, **Working rules**, and every **Design rules** subsection of `AGENTS.md`. Check required approvals against the available conversation; report missing evidence rather than assume approval. For **Layers**, inspect direct dependencies and complete boundary signatures, including nested types and errors, and verify ownership against that section.
4. Review documentation changes separately against **Documentation**.
5. If the change affects game-side code or its dependency graph, verify the **Layers** restrictions on `no_std`, `alloc`, and `std`, including affected transitive dependencies under the enabled features. Include dependency version and feature changes, not only new crates.
6. If the change touches an unsafe crate, check that every SAFETY contract proves soundness for all inputs a safe caller can supply, that the safe API cannot be misused to break it, and that its Miri tests exercise every unsafe block.
7. For changes to manifests, lockfiles, toolchain or lint configuration, check scripts, hooks, CI, agent rules, or this skill, trace the affected enforcement path. Confirm lint inheritance, exact toolchain pins, deployment-target coverage, unsafe-crate lint parity, and local/CI check parity against **Enforcement** and **Working rules**. Ensure rules are neither weakened nor duplicated and that this skill covers the rules left to review.
8. Report. Do not edit code unless the user asks for fixes.

Rules this review keeps catching that a higher layer could express are tooling gaps. So is anything a static layer should have caught but missed.

## Report

Group findings by kind. For each, cite `file:line`, the rule as `AGENTS.md › <subsection>`, why it applies, and a concrete fix:

- **Violation**: breaks a rule.
- **Concern**: a judgment call worth discussing.
- **Tooling gap**: something a higher enforcement layer could own.

If nothing applies, say "No findings." Do not praise or restate static-check results.
