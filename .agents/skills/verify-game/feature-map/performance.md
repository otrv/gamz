# Benchmarking and profiling

## Correctness before performance

The baseline helper still selects the regular `game`. The manifest-only `benchmark-platform` selects `benchmark-game` under the same dependency name and compiles the unchanged production host sources; keep its host dependencies aligned with `platform` when changing them. Normal builds do not select either benchmark package.

```sh
cargo build --locked -p benchmark-platform --features cli --bin benchmark-cli --target-dir target/verify-game
printf '%s\n' '{"dt":{"secs":0,"nanos":0},"controller":{}}' | target/verify-game/debug/benchmark-cli
```

Health-check each fresh process: require two complete texture uploads before supplying input, and zero status on EOF. Preserve complete output, not only counts or hashes. Compare two fresh processes given the same sequence: two zero-dt frames, enough positive-dt frames to cross entity recycling, zero dt again, and interleaved audio requests. Require identical replays, unchanged output during zero dt, changed positions during positive dt, and preserved sprite counts through recycling. On separate fresh states, 0.1 seconds and the maximum valid duration should match because the example caps elapsed time. Controller/back values should not change this example's output or terminate the CLI; requested audio frames are currently silent. These are example-specific checks, not new assertions for the regular-game helper.

For rendered pixels, run `cargo run --locked -p benchmark-platform --features linux --bin benchmark-linux`. Health-check the `gamz` window, adapter, and nonzero rendered-command log. Capture and inspect the populated scene, clipped sprites, text, and animation; then focus the window, send Escape, and require exit zero. Keep capture work separate from timed runs.

## Measure the normal platform

Build the current working tree, including uncommitted edits, in release mode from the repository root. There is no custom runner. The explicit native target and output directory avoid accidentally running a stale binary when Cargo has a different target configured; line tables support profiling.

```sh
native="$(rustc -Vv | sed -n 's/^host: //p')"
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --locked \
    -p benchmark-platform --features linux --bin benchmark-linux \
    --target "$native" --target-dir target/benchmark
```

Only after a successful build, measure one fresh process for ten seconds. Do not time `cargo run`:

```sh
binary="target/benchmark/$native/release/benchmark-linux"
/usr/bin/time -v timeout --kill-after=2s 10s "$binary"
```

Require GNU time, timeout, X11, a supported WGPU driver, and a working ALSA default device. On Debian, install `time` and optionally `linux-perf`. Only CPU profiling requires Linux perf and permission to collect performance events; do not change kernel permissions automatically if it fails. In an orb, an explicit ALSA null configuration and `xvfb-run -a -s '-screen 0 1280x720x24'` can supply test prerequisites; keep Xvfb startup outside GNU time and label software rendering and null audio in the report. They do not represent physical GPU/audio performance.

Choose the finite duration and repetition count before measuring; the example uses one ten-second run. Do not edit, build, capture, or run other tests during measurement. For comparisons, keep settings and environment identical and repeat enough to assess variability. Retain each run rather than only an average. All runs include startup, audio, normal frame pacing, and presentation waits. Filesystem/driver caches are not cleared. There is no internal benchmark mode, fixed-step graphical replay, isolated warmup measurement, GPU timer, or frame-rate measurement. The frame cap can hide rendering headroom. CPU seconds may exceed elapsed seconds when multiple threads execute.

The caller owns validation and evidence. Retain GNU time output, the command/duration and exit status, and game logs for each run. GNU time's `-o PATH` separates resource output from game diagnostics; shell redirection can capture the latter. Record the machine/CPU, OS, driver/adapter, display size, audio setup, toolchain/build settings, and source state including uncommitted edits so a comparison can be assessed. Use distinct paths under `.amp/in/artifacts/` for evidence shared with the user; keep temporary probes and raw profiles under `target/`.

Check the exit status immediately and inspect game logs for a healthy adapter and nonzero `commands=... quads=... batches=...` output. Exit 124 is the expected external timeout, not proof that the game rendered or a clean window-close test. Reject crashes, early exits, forced kills, missing rendering, or tool errors even if resource numbers were printed. If using optional perf counters, reject `<not supported>` or `<not counted>` rather than treating them as zero. Preserve failing evidence. Peak RSS is not per-function allocation or total GPU memory. Repeatability depends on machine load and caches; these are measurements, not automatic regression verdicts.

## Profile CPU execution

Record a separate user-space sampling pass with DWARF call stacks:

```sh
perf record -e cpu-clock:u -F 99 --call-graph dwarf,8192 \
    -o target/benchmark/perf.data -- timeout --kill-after=2s 10s "$binary"
perf report -i target/benchmark/perf.data
```

Inspect recording diagnostics and use `perf report -i PATH` (or `--stdio` for text) to require game samples, not just the timeout process, and inspect function/call-stack attribution. Keep the matching executable with retained profiles before rebuilding, and use distinct output paths for separate recordings. Samples cover startup and the whole run; absence of a function from a short sample does not establish negligible cost. There is no GPU or allocation attribution. Keep this pass out of ordinary measurement averages.

When changing these recipes, execute them on the real platform and check that a failed launch cannot be mistaken for a healthy measurement. Never add a performance gate to pre-commit or the correctness check.
