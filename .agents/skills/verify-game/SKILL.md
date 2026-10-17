---
name: verify-game
description: "Verifies game-layer changes through the CLI. Uses platform-specific checks only when that platform is altered. Use for game verification and performance measurements."
---

# Verify a gamz game

Use only the production CLI for game-layer changes. Supply complete `FrameInput` values or audio requests and inspect the returned uploads, frames, or samples. CLI checks need no display, GPU, or real-time waits.

Run platform-specific checks only when that specific platform is altered. Game-layer changes alone never justify Linux rendering, keyboard, window, or playback checks. Performance measurements and maintenance audits follow the same boundary. During maintenance, review every feature from source, exercise its CLI path, and mark unchanged-platform paths as outside the run's scope. Do not launch an unchanged platform to complete a feature-map checklist. `cargo xtask check` still covers all configured targets.

## Launch

Run from the repository root:

```sh
.agents/skills/verify-game/scripts/verify-game launch
```

This builds the default game through `game-entry` with locked dependencies into `target/verify-game`, without graphical features. Run on Linux with Python 3 and the pinned Rust toolchain. Each drive owns a fresh process and supplies steps serially. Relaunch to reset game state. Rebuild after source changes or after building another selection into the same target directory.

## Doctor

```sh
.agents/skills/verify-game/scripts/verify-game doctor
```

Doctor launches the built binary without display variables, checks its flushed initialization response, closes stdin, and requires exit status zero. Every drive checks its own process before sending input. After an unexpected result, retain the evidence, fix the cause, rebuild if needed, and run doctor again. Never continue a failed session.

## Drive

Read the relevant feature-map entries, then run:

```sh
.agents/skills/verify-game/scripts/verify-game drive
```

The baseline sends press, held, and released values for all twelve buttons, multiple transitions, and zero, fractional, and maximum timesteps. It waits for each flushed response before sending the next step. It compares replay output and checks the input-size boundary, EOF, and immediate error termination. Inspect the inputs, outputs, diagnostics, and statuses in the printed `.amp/in/artifacts/game-verification/run-*` directory. Error probes require the expected diagnostic and nonzero termination while stdin remains open.

After changing the helper, repeat `doctor` and `drive` with `PYTHONOPTIMIZE=1`. Verification and replay must still execute. Response reads have a 15-second deadline and a 272 MiB limit, independent of pipe fragmentation. The byte budget accommodates 64 MiB of textures expanded to JSON arrays plus metadata.

For a specific game path, drive the same binary with your own input sequence rather than changing the game or adding test-only hooks:

```sh
printf '%s\n' '{"dt":{"secs":0,"nanos":16666667},"controller":{"move_right":{"ended":"down","half_transitions":1}}}' | target/verify-game/debug/cli
```

Both `dt` and `controller` are required. Omitted buttons mean released with zero transitions for that frame. Supply a held button again with `ended: "down"` and `half_transitions: 0`. Supply release and multiple-transition counts explicitly. Asset paths resolve against the process working directory. Run from the intended asset root with real assets or caller-created fixtures. Use `--persistent-bytes N` and `--transient-bytes N` for alternate initialization capacities.

The first output line contains complete texture uploads. Each subsequent line contains a visual frame or audio samples. The game supplies texture IDs without caller acknowledgment. Commands include text glyph geometry. Observe complete outputs and relevant file effects, never opaque game memory. See `crates/platform/src/cli/input.rs` and `output.rs` for the JSON representation. Follow [Audio](feature-map/audio.md) for audio requests and playback. The baseline `drive` covers visual frames only.

The default game ignores inputs and returns an empty black frame. The baseline proves the CLI protocol, not gameplay responses or cross-machine determinism. When gameplay changes, update the feature map and baseline with concrete sequences and independently expected results. Follow [Benchmarking and profiling](feature-map/performance.md) for example selection and separate correctness checks. Baseline assertions do not apply to examples.

## Rendering and platform behavior

Only when the Linux platform is altered, run:

```sh
.agents/skills/verify-game/scripts/verify-game linux
```

This builds `linux` and uses a private Xvfb display for one bounded drive. It checks the window, adapter, and frame log, sends mapped keys, captures images, sends Escape, and requires exit status zero.

Install `xvfb-run`, `xauth`, `xdotool`, ImageMagick `import`, a usable WGPU driver, and a working ALSA `default` device. An explicitly configured ALSA null device supports window-only checks, not playback timing or underruns. Follow [Initialization and rendering](feature-map/launch-and-render.md) to inspect the images and log. Test window-manager close controls and focus or repeat behavior separately when affected.

## Performance

Measure game-layer changes through a fixed CLI workload before and after editing. Measure a platform only when that platform is altered. Follow [Benchmarking and profiling](feature-map/performance.md) for commands, environment controls, validation, evidence, and limitations. Documentation-only changes need no benchmark. Do not revert others' work to manufacture a baseline. Report missing or incompatible measurements as limitations, not proof of no regression. Say when the example does not execute the changed code and needs another CLI workload.

## Cleanup

Every drive closes or kills its owned process on completion or failure; the Linux wrapper also tears down Xvfb. No shared desktop or persistent service is used. Evidence survives cleanup:

```sh
.agents/skills/verify-game/scripts/verify-game cleanup
```

Before reporting, verify that named evidence files still exist. Preserve failing evidence and report limitations. Do not claim gameplay coverage from a successful empty frame.

## Feature map

Use the [feature-map index](feature-map/README.md) to find recipes and coverage limits. Keep entries current as game behavior grows. Add concrete inputs and expected outputs before claiming coverage of a new feature.
