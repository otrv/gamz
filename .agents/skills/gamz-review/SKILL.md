---
name: gamz-review
description: "Reviews gamz changes against AGENTS.md where static checks cannot enforce it. Use for /gamz-review, gamz diff, commit, or PR reviews, and before finishing any repository change."
---

# gamz review

Read [AGENTS.md](../../../AGENTS.md) for the rules. This skill defines how to review what static checks cannot enforce.

## Pre-change subtraction check

Before implementing additions involving crates, dependencies, public APIs, scripts, custom checks, abstractions, protocols, tests, or documentation, establish:

- The concrete defect or requirement.
- Why existing code, tooling, or review cannot own it without the addition.
- The smallest alternative considered, including removal or reuse.
- What the addition requires maintaining.
- Whether user approval is required under **Working rules**, and whether it was obtained.

For an existing diff, assess these against the pre-change state. Reject unjustified additions. Keep this judgment in review, not a custom script.

## Workflow

1. Review the paths, commits, or PR the user named. Otherwise use the diff against the merge base with `origin/main`, falling back to `main`, including uncommitted and untracked files. Apply the **Pre-change subtraction check**. Read whole touched functions, types, and modules when rules such as bounds or ownership need context.
2. Run `cargo xtask check`. If it fails, report that as the first finding and keep reviewing. Do not re-report anything the compiler, Clippy, rustfmt, or the runner detects.
3. Check every change against **Priorities**, **Enforcement**, **Working rules**, and every **Design rules** subsection. Check required approvals against the conversation and report missing evidence. For **Layers**, inspect direct dependencies and complete boundary signatures, including nested types and errors, to verify ownership.
   Reject handwritten local lint suppressions static checks miss, including suppressions of the four macro-compatible `deny` lints, allowances for centrally allowed lints, and attributes in unchecked configurations. Third-party macro-generated allowances are not handwritten suppressions.
4. Review documentation changes separately against **Documentation**.
5. For game-side code or dependency changes, verify the **Layers** restrictions on `no_std`, `alloc`, and `std` through affected transitive dependencies and enabled features. Include version and feature changes, not only new crates.
   For added or expanded `game-core` functionality, enforce the **Layers** example requirement with execution evidence from the relevant CLI or graphical platform. Trace each addition to its example, runtime path, and observable behavior. Reject evidence whose demonstrated behavior would be unchanged if the addition did nothing.
   For example changes, enforce **Layers** isolation by inspecting code, asset paths, and build inputs, including shared helpers, `#[path]`, `include!`, and `include_bytes!`. Check every example directory's registration through `game-entry` and compilation through both hosts. Do not replace absent modules with public sibling modules or introduce a game interface outside the trait.
6. Apply **Testing** to unit-test scope and quality. Require production-CLI evidence for game changes and execution of the affected platform for platform changes. For unsafe crates, check that every SAFETY contract proves soundness for every input a safe caller can supply. Verify that the safe API cannot violate the contract and Miri exercises every unsafe block.
7. For manifests, lockfiles, toolchain or lint configuration, check tooling, hooks, CI, agent rules, or this skill, trace the affected enforcement path. Confirm lint inheritance, exact toolchain pins, deployment-target coverage, unsafe-crate lint parity, and parity between local and CI checks against **Enforcement** and **Working rules**. Ensure rules are neither weakened nor duplicated and this skill covers those left to review.
   When upgrading Rust, review the default-deny lint inventory against `rustc -W help`.
8. Report. Do not edit code unless the user asks for fixes.

Report recurring findings that a higher layer could enforce, and anything static checks should have caught, as tooling gaps.

## Report

Group findings by kind. Cite `file:line`, the rule as `AGENTS.md › <subsection>`, why it applies, and a concrete fix for each:

- **Violation**: breaks a rule.
- **Concern**: a judgment call worth discussing.
- **Tooling gap**: something a higher enforcement layer could own.

If nothing applies, say "No findings." Do not praise or restate static-check results.

When performance impact is relevant, include before and after measurements and evidence, or state what remains unmeasured. Follow `verify-game` for runtime performance.

For implementation work, end the completion report with this receipt. Omit it for review-only requests.

```text
cargo xtask check: <result>
/gamz-review: <scope>; <result>
runtime verification: <command and artifact links, or not applicable>
```
