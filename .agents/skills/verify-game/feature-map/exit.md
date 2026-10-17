# Exit and failure behavior

## CLI

EOF exits successfully after initialization and any supplied updates. Invalid input, out-of-range values, oversized lines, initialization failure, or I/O errors produce a stderr diagnostic and nonzero exit status. There is no quit request or recovery dialogue. `controller.back` is game input, not a host-exit instruction.

Run `drive`. Require zero status for `controls`, `replay`, `at-limit`, and `eof` sessions. Require nonzero status for `invalid` and `oversized` sessions. The size probes pad valid JSON to 16,384 and 16,385 bytes, including the newline. The first must return a frame. The second must report the size-limit diagnostic and exit while stdin remains open, without a frame response. Inspect the recorded JSONL, stderr, and statuses. Each session checks initialization before input.

For initialization failure, run `target/verify-game/debug/cli --persistent-bytes 0 < /dev/null`. Require no stdout, a memory diagnostic on stderr, and nonzero exit status. Run doctor normally before further testing.

## Check window exit only when Linux is altered

Escape, a window-close request, and window destruction exit the event loop. Run the helper's `linux` drive to verify Escape after recording other evidence. Require `linux-status.txt` to contain zero. Window-manager close controls need a desktop session and a fresh run. The private Xvfb helper tests Escape only. A forced kill does not prove clean window-close behavior.

All helper-owned processes are torn down on success or failure. Evidence remains in the printed run directory.
