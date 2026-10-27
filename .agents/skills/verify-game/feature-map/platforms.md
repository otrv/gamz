# Verify an affected platform

Select hosts using the [verification scope in AGENTS.md](../../../../AGENTS.md#verify-behavior-and-evidence), including consumers of a changed game-host API. Exercise the affected platform contract, not game-rule correctness through visible gameplay.

## Linux

Build with `cargo build -p platform-linux --no-default-features --features linux --bin linux --locked`. Use an X11 session, a usable GPU adapter, and an ALSA `default` device. A bounded private Xvfb session can test window presentation; clean up every process it starts.

Exercise only affected properties: window creation and presentation, resize or minimize recovery, key mapping and focus-loss release, repeat suppression, or close-event delivery. Escape and window-close requests deliver `PlatformEvent::CloseRequested` to the game, which chooses whether to call the platform's `exit` function pointer. A destroyed window forces host exit. A clear-only frame is valid. Inspect screenshots and logs for rendering changes, and record input interactions when relevant. Nonzero command counts and game-specific pixels are not host readiness requirements.

For audio changes, use a real device or clocked virtual sink and compare captured samples from an exercised, independently known signal. An ALSA null device proves neither playback timing nor underrun recovery. For recovery, stop the process for one second, resume it, and require sound afterward. Test unavailable-device failure when affected. Do not change the game to emit a test signal without approval.

Sources: `crates/platform-linux/src/linux/{main,audio,input}.rs`, `crates/platform-linux/src/{files,window,limits}.rs`, and `crates/renderer-wgpu/src/wgpu/`.

Keep complete platform diagnostics and relevant captures. Exercise texture, clipping, sampling, or audio paths only with suitable existing content or an explicitly authorized probe. Mark missing prerequisites and unexercised paths rather than launching unrelated hosts or claiming coverage from process survival.
