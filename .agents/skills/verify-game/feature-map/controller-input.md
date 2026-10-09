# Controller input and timesteps

## Game path: CLI

Supply `dt: {secs, nanos}` and `controller` in each input line. All twelve named buttons in `ControllerInput` are supported, including `back`; each has final position and a transition count. Omitted buttons reset to up/zero. A press, held frame, and release require `down/1`, `down/0`, and `up/1` respectively. A press/release within one frame can be supplied as `up/2`. Time is caller-controlled, not inferred from delays between lines.

Run `launch`, `doctor`, then `drive`. Inspect `controls-input.jsonl`, `controls-output.jsonl`, and the identical `replay-output.jsonl` in the printed run directory. The baseline sends all buttons, varied transition counts, and zero/fractional/maximum durations. Every input produces one empty black frame; `back` does not terminate the CLI.

The current game ignores input. This live run proves complete-input acceptance and frame-loop operation, not game interpretation of every field. Add asymmetric, observable gameplay expectations when the game begins using input.

## Platform path: Linux only when affected

`WASD` map to movement, arrows to action directions, `Q/E` to shoulders, Space to start. Escape maps to back but also exits the host. Linux accumulates transitions, retains held positions between frames, ignores repeated keydown events, and releases buttons on focus loss.

Run the helper's `linux` drive for mapped-key smoke coverage. Read its action transcript and inspect before/after images. If changing held/repeat/focus behavior, perform those actions explicitly in a graphical session; the baseline key taps do not prove them. CLI values do not verify OS event mapping.
