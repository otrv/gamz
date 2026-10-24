---
name: verify-game
description: "Verifies engine contracts and game changes through the production CLI, independently of the linked game. Use for runtime verification, CLI performance measurements, and selecting platform-only checks."
---

# Verify engine contracts through the CLI

Use the production CLI for all game behavior and game-performance checks. The helper builds and launches only that host. Its baseline checks engine transport, not the correctness of any game.

Keep this skill limited to engine contracts. Supply gameplay workloads and expected results for the current task, outside this directory, so replacing `crates/game` does not require rewriting verification tooling.

## Launch and check health

Run on Linux with Python 3.11 or newer and the pinned Rust toolchain:

```sh
.agents/skills/verify-game/scripts/verify-game launch
.agents/skills/verify-game/scripts/verify-game doctor
.agents/skills/verify-game/scripts/verify-game drive
```

`launch` builds the CLI with locked dependencies and no default features into `target/verify-game`. `doctor` checks initialization and EOF. `drive` checks the [CLI contract](feature-map/cli.md), then reports fresh-process replay equality without assuming determinism.

Each session starts a fresh process without display variables, checks flushed initialization before input, and sends requests serially. After any unexpected result, preserve evidence, fix the cause, rebuild if needed, and run doctor before another drive.

Use `--cwd PATH` for the asset root and `--persistent-bytes N` or `--transient-bytes N` when the game needs different capacities. Supply the same options to doctor and drive. Initialization requires the game's real assets and sufficient memory; failure is not permission to weaken the checks.

## Verify the changed behavior

Read the affected engine contract and its current game caller. Choose inputs that distinguish the intended result from a plausible bug. Write independent expected observations before running, including boundary cases and an unaffected control.

```sh
.agents/skills/verify-game/scripts/verify-game drive --inputs target/requests.jsonl
```

The file replaces the generic baseline. It contains valid frame or audio requests, one per line. For exact expectations, add `--expected target/expected.jsonl`. That file must contain every output record, including startup. Both the first run and replay must match its parsed JSON values completely. Do not generate expectations by accepting the executable's current output.

When exact equality is inappropriate, inspect the complete captured outputs with task-specific assertions and justified numeric tolerances. A successful baseline or identical replay is not evidence that gameplay, input interpretation, or an unused engine mechanism works. Report unexercised paths; do not add dummy game calls or test-only product hooks to reach them.

## Evidence and cleanup

Each printed `.amp/in/artifacts/game-verification/run-*` directory retains complete inputs, outputs, stderr, exit statuses, executable hash, working directory, and memory arguments. Record required asset contents and source state alongside task-specific evidence. Response and exit deadlines are 15 seconds; each response is capped at 272 MiB. These are helper budgets, not game timing guarantees.

Every session reaps its process on success or failure. `cleanup` confirms this ownership model and keeps evidence. Check evidence paths before reporting. After editing the helper, repeat doctor and drive with `PYTHONOPTIMIZE=1`; assertions must still execute.

## Select other checks by the changed layer

- Run `.githooks/pre-commit` for affected static checks, or `--all` for the full suite. Existing utility tests and Miri are separate from game verification.
- Use [CLI performance measurements](feature-map/performance.md) only when performance is in scope.
- Read [platform checks](feature-map/platforms.md) only to verify affected host or renderer functionality. Never launch a graphical host to test game rules or complete a maintenance checklist. Skill-only maintenance does not trigger platform runs.

The [feature map](feature-map/README.md) records engine boundaries and coverage limits, not the bundled game's behavior.
