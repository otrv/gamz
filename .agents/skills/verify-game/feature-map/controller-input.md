# Controller input and timesteps

## Check controller values through the CLI

Supply `dt: {secs, nanos}` and `controller` in each input line. Each of the twelve buttons in `ControllerInput`, including `back`, has `ended` and `half_transitions` fields. Omitted buttons reset to up with zero transitions. Supply these values for each state:

| State | `ended` | `half_transitions` |
| --- | --- | --- |
| Press | `down` | 1 |
| Hold | `down` | 0 |
| Release | `up` | 1 |
| Press and release within one frame | `up` | 2 |

The caller supplies time. Delays between lines do not change `dt`.

Run `launch`, `doctor`, then `drive`. Inspect `controls-input.jsonl`, `controls-output.jsonl`, and the identical `replay-output.jsonl` in the printed run directory. The baseline sends all buttons, varied transition counts, and zero, fractional, and maximum durations. Every input produces one empty black frame. `back` does not terminate the CLI.

The default game ignores input. This run proves input acceptance and frame-loop operation, not game interpretation of each field. Add asymmetric gameplay expectations when the game begins using input.

## Check keyboard integration only when Linux is altered

`WASD` maps to movement, arrow keys to action directions, `Q` and `E` to shoulders, and Space to start. Escape maps to back and exits the host. Linux accumulates transitions, retains held positions between frames, ignores repeated keydown events, and releases buttons on focus loss.

Run the helper's `linux` drive to check mapped-key delivery without a crash. Read its action transcript and inspect both images. The default game has no visible response to those keys. If held, repeat, or focus behavior changes, test those actions explicitly with observable output in a graphical session. Baseline key taps and CLI values do not prove OS event mapping.
