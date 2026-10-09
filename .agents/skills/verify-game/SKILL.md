---
name: verify-game
description: Run the current gamz game through the Linux platform, exercise its keyboard controls, and preserve visual and runtime proof.
---

# Verify a gamz game

Use this skill after changing user-visible game behavior in `crates/game`. It drives the production `linux` binary rather than calling game functions directly. Unit tests remain appropriate for pure game rules, but they do not replace this run.

## Launch

The game is a Linux/X11 application. The checked-in helper builds the platform, then creates a private Xvfb display in a tmux session and waits for the `gamz` window:

```sh
.agents/skills/verify-game/scripts/verify-game launch
```

It uses `cargo run -p platform --features linux --bin linux --locked`, so the pinned Rust toolchain and the project's locked dependencies are required. The game is ready only when the command prints a `ready:` line with a window ID. Do not use an existing desktop window or start a second verifier at the same time: the default run owns X display `:99` and tmux session `gamz-verify-$UID`.

## Doctor

Before driving, or whenever a run behaves unexpectedly, check the instance that this skill started:

```sh
.agents/skills/verify-game/scripts/verify-game doctor
```

It confirms the owned tmux session, the `gamz` window on the owned display, and a WGPU adapter line in that run's log. A failed doctor means do not send input. Run cleanup, inspect the named log if needed, and launch again.

## Drive

Read every relevant feature-map entry before choosing coverage. The helper exercises every platform keyboard binding (`WASD`, arrow keys, `Q`, `E`, Space, Escape), captures the game before Escape, records the action order, and copies its runtime log:

```sh
.agents/skills/verify-game/scripts/verify-game drive
```

Evidence is retained under `.amp/in/artifacts/game-verification/` even after cleanup. For a changed feature, inspect both PNGs and the action transcript. Prove the real user path: capture the input that causes the result as well as its visible result. Also verify side effects such as files, persistent state, or log output when the feature has them. Do not use game internals, test-only entry points, or mocks to claim platform behavior works.

The current boilerplate game deliberately clears to opaque black and has no gameplay response yet. Its proof is therefore a live `gamz` window that stays running after all mapped inputs, a black rendered frame before and after, and runtime stats in the copied log. When a game gains visible behavior, replace this baseline expectation in the relevant feature-map files with concrete screen and state expectations.

## Cleanup

Always tear down the owned instance, including after a failed attempt:

```sh
.agents/skills/verify-game/scripts/verify-game cleanup
```

Cleanup kills only the tmux session that the helper created and removes its scratch logs. It never removes `.amp/in/artifacts/game-verification/` proof files.

## Feature map

Read the index and all entries affected by the change. Keep this map current as `crates/game` gains user-facing behavior; add a feature file before claiming that new behavior has been fully verified.

- [Feature-map index](feature-map/README.md)
- [Launching and rendering](feature-map/launch-and-render.md)
- [Controller input](feature-map/controller-input.md)
- [Exit behavior](feature-map/exit.md)
