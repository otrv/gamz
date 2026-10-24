# Check the production CLI contract

Run the helper's `launch`, `doctor`, and `drive`. The baseline checks the following boundaries without prescribing game output.

| Boundary | Required observation |
| --- | --- |
| Initialization | One flushed `uploads` array before input. Empty and nonempty uploads are valid. Slots, dimensions, RGBA lengths, and byte budgets are checked. |
| Frames | One response per request with valid canvas dimensions, byte clear color, bounded command list, and known command tags. Empty command lists are valid. |
| Controller and time | All twelve button fields accept press, hold, release, multiple transitions, and the maximum `u32` count. Durations include zero, fractional, and maximum values. |
| Audio | Requested rate and number of stereo i16 pairs, including zero and maximum length. Silence and nonzero samples are valid. |
| Framing | A 16,384-byte line, including newline, succeeds. A valid final record without newline is processed at EOF. |
| Rejection | Missing, null, unknown, mixed, or out-of-range input exits 1 with a relevant diagnostic while stdin stays open. Oversized input fails with or without newline. A queued valid record after malformed input is not processed. |
| EOF | Exit 0, no diagnostics, and no extra output. Every session is reaped and its evidence retained. |

The baseline validates response envelopes, upload bytes, audio samples, and render-command tags. It captures every command field but does not implement a second renderer or assert command geometry. For affected commands, inspect `crates/platform-linux/src/cli/output.rs` and compare complete captured values against independently derived expectations.

Frame requests contain `dt: {secs, nanos}` and `controller`. Omitted buttons become up with zero transitions. Explicit buttons contain `ended: "up"` or `"down"` and `half_transitions: u32`. `secs` is `u64`; `nanos` is less than 1000000000. The host does not cap time or infer transitions. `back` does not exit the CLI. Accepted input alone does not prove its interpretation by the game or OS event accumulation.

Audio requests are `{"audio":{"sample_rate":48000,"frames":8}}`. Rates span 1–192000 Hz and counts span 0–16384 stereo frames. Channels span −32768–32767. Audio cannot share a record with frame fields. Both frame and audio callbacks may mutate game state, even with zero duration or zero samples. Repeatability, silence, and unchanged frames are not engine guarantees.

Source: `crates/platform-linux/src/cli/{main,input,output}.rs` and `crates/platform-api/src/{lib,input,timing,audio,render}.rs`. Asset reads use `crates/platform-linux/src/linux/services.rs` relative to `--cwd`; they require an exercised game path. Do not infer memory failure from zero capacity: a game may need no persistent storage. For host argument rejection, use `--persistent-bytes 1073741825` and require exit 1, no stdout, and the memory-size diagnostic.
