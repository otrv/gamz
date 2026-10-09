---
name: verify-game
description: "Verifies gamz game behavior through controlled CLI inputs and complete initialization/frame outputs. Uses the graphical Linux platform only for rendering, keyboard/window integration, or other behavior outside the game."
---

# Verify a gamz game

Use the production `cli` entrypoint by default for game behavior. Supply complete `FrameInput` values or explicit audio requests and inspect initialization uploads and returned frames or samples. Do not require a window, GPU, screenshots, or wall-clock timing for game tests. Use another platform only when the behavior under test is outside the game boundary: rendering, platform services, keyboard/window integration, or audio playback.

## Launch

Run from the repository root:

```sh
.agents/skills/verify-game/scripts/verify-game launch
```

This builds the CLI with locked dependencies and no graphical features into `target/verify-game`. Linux, Python 3, and the pinned Rust toolchain are required; X11 and a GPU are not. Each drive owns a fresh process, supplies steps serially, then closes stdin. Relaunch a process to reset game state. Re-run `launch` after source changes.

## Doctor

```sh
.agents/skills/verify-game/scripts/verify-game doctor
```

Doctor launches the built binary without display variables, checks its flushed initialization response, closes stdin, and requires exit status zero. Every drive also checks initialization on its own process before sending input. After an unexpected result, retain the evidence, fix the cause, rebuild if needed, and run doctor again. Never continue a failed session.

## Drive

Read the relevant feature-map entries, then run:

```sh
.agents/skills/verify-game/scripts/verify-game drive
```

The baseline drive sends press, held, and released values for all twelve buttons, multi-transition input, and zero/fractional/maximum timesteps. It requires a flushed response before sending the next step, compares replay output, checks both sides of the input-size limit, EOF, and fail-fast errors, and records inputs, outputs, diagnostics, and exit statuses under the printed `.amp/in/artifacts/game-verification/run-*` directory. Error probes require the expected diagnostic and nonzero termination while stdin remains open.

After changing the helper, repeat `doctor` and `drive` with `PYTHONOPTIMIZE=1`; verification and replay must still execute. Response reads have a 15-second deadline and a 272 MiB byte limit, independent of pipe fragmentation. That byte budget accommodates the 64 MiB texture budget expanded to JSON byte arrays plus metadata.

For a specific game path, drive the same binary with your own input sequence rather than changing the game or adding test-only hooks:

```sh
printf '%s\n' '{"dt":{"secs":0,"nanos":16666667},"controller":{"move_right":{"ended":"down","half_transitions":1}}}' | target/verify-game/debug/cli
```

Both `dt` and `controller` are required. Omitted buttons mean released with zero transitions **for that frame**; a held button must be supplied again with `ended: "down"` and `half_transitions: 0`. Supply release and multiple-transition counts explicitly. No hidden steps or real-time waits are needed. Asset paths resolve against the process working directory, so run from the intended asset root; use real assets or caller-created fixture files. `--persistent-bytes N` and `--transient-bytes N` exercise initialization with alternate capacities.

The first output line contains complete texture uploads; each subsequent line is the returned visual frame or audio samples. Texture IDs are supplied by the game, not acknowledged by the caller. Commands contain all fields, including text glyph geometry. Observe these outputs and any relevant file effects; never inspect opaque game memory. See `crates/platform/src/cli/input.rs` and `output.rs` for the wire representation. For manual audio requests and playback verification, follow [Audio](feature-map/audio.md); the baseline `drive` covers visual frames only.

The current game ignores inputs and returns an empty black frame. The baseline therefore proves the transport, not gameplay responses or cross-machine determinism. When gameplay is added, update the feature map and baseline assertions with concrete sequences and independently expected results.

## Rendering and platform behavior

Only when the affected feature needs it, run:

```sh
.agents/skills/verify-game/scripts/verify-game linux
```

This builds the `linux` binary, owns a private Xvfb display for one bounded drive, checks the window/adapter/frame log, exercises mapped keyboard input, captures before/after images, sends Escape, and requires exit status zero. It requires `xvfb-run`, `xauth`, `xdotool`, ImageMagick `import`, a usable WGPU driver, and a working ALSA `default` playback device. An explicitly configured ALSA null device is sufficient for window-only checks, but does not verify playback timing or underruns. Inspect both printed-directory PNGs with `view_media` and inspect the log. CLI render commands do not prove rendered pixels. Window-manager close controls and focus/repeat behavior need their own live graphical actions when affected; the helper does not claim to test them.

## Cleanup

Every drive closes or kills its owned process on completion or failure; the Linux wrapper also tears down Xvfb. No shared desktop or persistent service is used. Evidence survives cleanup:

```sh
.agents/skills/verify-game/scripts/verify-game cleanup
```

Before reporting, verify the named evidence files still exist. Preserve failing evidence and state limitations rather than replacing missing gameplay behavior with success assertions.

## Feature map

Keep entries current as game behavior grows. Add a concrete input sequence and observable expectation before claiming coverage of a new feature.

- [Feature-map index](feature-map/README.md)
- [Initialization, frame output, and rendering](feature-map/launch-and-render.md)
- [Controller input and timesteps](feature-map/controller-input.md)
- [Audio generation and playback](feature-map/audio.md)
- [Exit and failure behavior](feature-map/exit.md)
