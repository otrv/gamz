# Check the production CLI contract

Run the helper's `launch`, `doctor`, and `drive`. Check the following boundaries without prescribing game output, with task-specific assertions for uploads and service requests.

| Boundary | Required observation |
| --- | --- |
| Initialization | One flushed `{"uploads":[]}` before input. Startup uploads are always empty. |
| Frames | One response per processed request with valid canvas dimensions, byte clear color, bounded command list, and known command tags. Empty command lists are valid. Frame responses may include `uploads`, omitted when empty. Check upload slots, dimensions, RGBA lengths, and byte budgets when exercised. |
| Controller and time | All twelve button fields accept press, hold, release, multiple transitions, and the maximum `u32` count. Durations include zero, fractional, and maximum values. |
| Audio | Requested rate and number of stereo i16 pairs, including zero and maximum length. Silence and nonzero samples are valid. |
| Framing | A 16,384-byte line, including newline, succeeds. A valid final record without newline is processed at EOF. |
| Rejection | Missing, null, unknown, mixed, or out-of-range input exits 1 with a relevant diagnostic while stdin stays open. Oversized input fails with or without newline. A queued valid record after malformed input is not processed. |
| EOF | Exit 0, no diagnostics, and no extra output. Every session is reaped and its evidence retained. |

The baseline validates response envelopes, audio samples, and render-command tags. It captures every command field but does not implement a second renderer or assert command geometry. For affected commands and uploads, inspect `crates/platform-linux/src/cli/output.rs` and compare complete captured values against independently derived expectations.

Frame requests contain `dt: {secs, nanos}` and `controller`. Omitted buttons become up with zero transitions. Explicit buttons contain `ended: "up"` or `"down"` and `half_transitions: u32`. `secs` is `u64`; `nanos` is less than 1000000000. The host does not cap time or infer transitions. `back` does not directly exit the CLI. Accepted input alone does not prove its interpretation by the game or OS event accumulation.

Frame requests also accept an optional `close_requested` boolean, which defaults to `false`. A `true` value delivers `PlatformEvent::CloseRequested`; the game chooses whether to call `Platform::request_exit`. The CLI flushes the current response before exiting successfully on that request. Exercise event handling with a task-specific workload rather than assuming that the event forces exit.

Audio requests are `{"audio":{"sample_rate":48000,"frames":8}}`. Rates span 1 to 192000 Hz and counts span 0 to 16384 stereo frames. Channels span -32768 to 32767. Audio cannot share a record with frame fields, including `close_requested`. Both frame and audio callbacks may mutate game state, even with zero duration or zero samples. Repeatability, silence, and unchanged frames are not engine guarantees.

Asynchronous reads and writes use `crates/platform-linux/src/linux/services.rs` relative to the helper's `--cwd`. Use disposable files and an exercised game path to check request submission, completion status, transferred bytes, and file contents. Continue frame requests until the expected completion is observed within a bounded workload; completion timing is not a fixed frame count. The host calls `Services::poll_events` before the game update. Read destinations remain unavailable and write sources remain immutable until completion, including after a cancellation request. Assert those reservations and their release through the exercised path when affected. Input acceptance alone does not prove service behavior.

Sources: `crates/platform-linux/src/cli/{main,input,output}.rs`, `crates/platform-linux/src/memory.rs`, `crates/platform-linux/src/linux/services.rs`, and `crates/platform-api/src/{game,platform,services,input,timing,audio,render}.rs`. Do not infer memory failure from zero capacity: a game may need no persistent storage. For host argument rejection, use `--persistent-bytes 1073741825` and require exit 1, no stdout, and the memory-size diagnostic.
