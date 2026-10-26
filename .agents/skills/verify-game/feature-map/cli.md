# Check the production CLI contract

Run the helper's `launch`, `doctor`, and `drive`. Check the following boundaries without prescribing game output, with task-specific assertions for texture and file requests.

| Boundary | Required observation |
| --- | --- |
| Initialization | One flushed object containing `uploads` before input. Uploads requested during game initialization appear here. |
| Frames | One response per processed request with positive finite canvas dimensions, normalized clear color, bounded command list, and known command tags. Empty command lists are valid. Frame responses may include callback `uploads` and `releases`, omitted when empty. Check upload slots, dimensions, RGBA lengths, release IDs, and byte budgets when exercised. |
| Controller and time | All twelve button fields accept press, hold, release, multiple transitions, and the maximum `u32` count. Durations include zero, fractional, and maximum values. |
| Audio | Requested rate and number of stereo i16 pairs, including zero and maximum length. Silence and nonzero samples are valid. |
| Framing | A 16,384-byte line, including newline, succeeds. A valid final record without newline is processed at EOF. |
| Rejection | Missing, null, unknown, mixed, or out-of-range input exits 1 with a relevant diagnostic while stdin stays open. Oversized input fails with or without newline. A queued valid record after malformed input is not processed. |
| EOF | Exit 0, no diagnostics, and no extra output. Every session is reaped and its evidence retained. |

The baseline validates response envelopes, audio samples, and render-command tags. It captures every command field but does not implement a second renderer or assert command geometry. For affected commands and uploads, inspect `crates/platform-linux/src/cli/output.rs` and compare complete captured values against independently derived expectations.

Frame requests contain `dt: {secs, nanos}` and `controller`. Omitted buttons become up with zero transitions. Explicit buttons contain `ended: "up"` or `"down"` and `half_transitions: u32`. `secs` is `u64`; `nanos` is less than 1000000000. The host does not cap time or infer transitions. `back` does not directly exit the CLI. Accepted input alone does not prove its interpretation by the game or OS event accumulation.

Controller `movement` and `aim` accept two finite components between -1 and 1, defaulting to zero. Frame fields `pointer` and `scroll` accept finite pairs; omission means no pointer and zero scroll. `text` accepts up to 4096 UTF-8 bytes, defaulting to empty. Explicit null is rejected. Physical device codes do not enter the contract.

Frame requests also accept an optional `close_requested` boolean, which defaults to `false`. A `true` value delivers `PlatformEvent::CloseRequested`; the game chooses whether to call the `Platform::exit` function pointer. The CLI flushes the current response before exiting successfully on that request. Exercise event handling with a task-specific workload rather than assuming that the event forces exit.

Audio requests are `{"audio":{"sample_rate":48000,"frames":8}}`. Rates span 1 to 192000 Hz and counts span 0 to 16384 stereo frames. Channels span -32768 to 32767. Audio cannot share a record with frame fields. Both frame and audio callbacks may mutate game state, even with zero duration or zero samples. Repeatability, silence, and unchanged frames are not engine guarantees.

Asynchronous reads and writes use `crates/platform-linux/src/files.rs` relative to the helper's `--cwd`. Use disposable files and an exercised game path to check request submission, completion status, transferred bytes, and file contents. Continue frame requests until the expected completion is observed within a bounded workload; completion timing is not a fixed frame count. The host collects completed buffers before the game update and recycles them afterward. Read events borrow result bytes for that update. Submission copies write bytes and paths; the game may reuse its source storage immediately. Check those lifetimes and completion results when affected. Input acceptance alone does not prove file behavior.

Sources: `crates/platform-linux/src/cli/{main,input,output}.rs`, `crates/platform-linux/src/{memory,limits,files,textures,window}.rs`, and `crates/platform-api/src/{game,platform,file,request,window,input,audio,render}.rs`. CLI limits are Linux policy, not cross-platform guarantees. Do not infer memory failure from zero capacity: a game may need no persistent storage. For host argument rejection, use `--persistent-bytes 1073741825` and require exit 1, no stdout, and the memory-size diagnostic.
