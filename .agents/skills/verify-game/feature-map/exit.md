# Exit and failure behavior

## CLI

EOF exits successfully after initialization and any supplied updates. Invalid input, out-of-range values, oversized lines, initialization failure, or I/O errors terminate the process with a nonzero exit status and stderr diagnostic. There is no quit request or recovery dialogue. `controller.back` is game input, not a host-exit instruction.

Run `drive`. Require zero status for controls/replay/at-limit/EOF sessions and nonzero status for invalid/oversized sessions. The size probes use otherwise-valid JSON padded to 16,384 and 16,385 bytes including newline: the former must return a frame, and the latter must report the size-limit diagnostic. Error sessions must terminate while stdin remains open, with no frame response. Inspect the recorded JSONL, stderr, and status files; do not infer success merely from process disappearance. Each session starts with its own initialization health check.

For initialization failure, run `target/verify-game/debug/cli --persistent-bytes 0` with closed stdin: require no stdout, a memory diagnostic on stderr, and nonzero exit status. Relaunch normally and run doctor before further testing.

## Linux platform

Escape press, window-close request, and window destruction exit the event loop. Run the helper's `linux` drive to verify Escape after recording other evidence; require `linux-status.txt` to contain zero. Window-close controls require a window manager/desktop session and a separate fresh run; the private Xvfb helper tests Escape only. Do not use force-kill as evidence of clean window-close behavior.

All helper-owned processes are torn down on success or failure. Evidence remains in the printed run directory.
