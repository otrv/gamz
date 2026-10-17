# Benchmarking and profiling

Use only the CLI for game-layer measurements. Run Linux measurements only when the Linux platform is altered. Documentation-only changes need no benchmark. When maintaining these recipes, execute the CLI recipe and leave unchanged-platform paths outside the run's scope.

## Check correctness before measuring

The baseline helper selects `game`. Use `game-entry/example-benchmark` to select `crates/examples/benchmark` through the production CLI and `platform-api::Game` trait. Disable default features because conflicting selections fail to compile. Preserve the baseline executable with a separate target directory:

```sh
cargo build --locked -p platform --no-default-features \
	--features cli,game-entry/example-benchmark --bin cli --target-dir target/verify-example
printf '%s\n' '{"dt":{"secs":0,"nanos":0},"controller":{}}' | target/verify-example/debug/cli
```

Check each fresh process before input. Require two complete texture uploads and exit status zero on EOF. Preserve complete correctness output, not just counts or hashes.

Compare two fresh processes given the same sequence. Send two zero-dt frames, enough positive-dt frames for entity recycling, zero dt again, and interleaved audio requests. Require identical replays, unchanged output during zero dt, changed positions during positive dt, and unchanged sprite counts through recycling. Compare fresh states given 0.1 seconds and the maximum valid duration. Their output must match because the example caps elapsed time. Controller values, including back, must not change output or terminate the CLI. Audio requests must return silence. Keep example-specific assertions out of the regular-game helper.

## Measure a fixed CLI workload

Build the current working tree, including uncommitted edits, from the repository root. The native target and explicit output directory avoid stale binaries when Cargo has another target configured. Line tables support profiling.

```sh
native="$(rustc -Vv | sed -n 's/^host: //p')"
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --locked \
	-p platform --no-default-features --features cli,game-entry/example-benchmark --bin cli \
	--target "$native" --target-dir target/benchmark
```

Prepare input outside the measurement. The following workload advances 6,000 frames with fixed timesteps. It needs no display, GPU, or audio device:

```sh
python3 - <<'PY'
from pathlib import Path
request = '{"dt":{"secs":0,"nanos":16666667},"controller":{}}\n'
Path('target/benchmark/frames.jsonl').write_text(request * 6000)
PY
binary="target/benchmark/$native/release/cli"
/usr/bin/time -v "$binary" < target/benchmark/frames.jsonl > /dev/null
```

Time the executable, not `cargo run` or input generation. Require exit status zero and no diagnostics. Before timing, run the release executable with a short controlled sequence and check its complete uploads and frame output as above. A failed launch can still print resource numbers. Reject that run rather than interpreting those numbers as performance.

The measurement includes startup, game updates, JSON parsing, and output serialization. Discarding timed stdout avoids disk-write cost but does not remove serialization cost. It does not measure GPU rendering, playback, graphical frame pacing, or isolated game-update cost. Use another CLI workload when the example does not execute the changed code. Do not launch an unchanged platform to fill that gap.

## Measure Linux only when Linux is altered

For populated rendering checks, build and run the example through the Linux host:

```sh
cargo run --locked -p platform --no-default-features \
	--features linux,game-entry/example-benchmark --bin linux --target-dir target/verify-example
```

Check the `gamz` window, adapter, and nonzero rendered-command log. Capture and inspect the populated scene, clipped sprites, text, and animation. Focus the window, send Escape, and require exit status zero. Keep captures separate from timed runs.

For measurements, build the release executable before timing:

```sh
native="$(rustc -Vv | sed -n 's/^host: //p')"
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release --locked \
	-p platform --no-default-features --features linux,game-entry/example-benchmark --bin linux \
	--target "$native" --target-dir target/benchmark
binary="target/benchmark/$native/release/linux"
/usr/bin/time -v timeout --kill-after=2s 10s "$binary"
```

Require X11, a supported WGPU driver, and a working ALSA default device. In an orb, use an explicit ALSA null configuration and `xvfb-run -a -s '-screen 0 1280x720x24'` when needed. Keep Xvfb startup outside GNU time. Label software rendering and null audio because they do not represent physical GPU or audio performance.

Inspect logs for a healthy adapter and nonzero `commands=... quads=... batches=...` output. Exit 124 means the timeout expired, not that rendering worked or the window closed cleanly. Reject early exits, crashes, forced kills, missing rendering, and tool errors.

Linux runs include startup, audio, frame pacing, and presentation waits. There is no fixed-step graphical replay, isolated warmup, GPU timer, or frame-rate measurement. The frame cap can hide unused rendering capacity.

## Keep measurements comparable

Install GNU time, and optionally Linux perf for profiling. On Debian, the packages are `time` and `linux-perf`. Choose a bounded workload and repetition count before measuring. For Linux, also choose a finite duration. Do not edit, build, capture, or run tests during measurement. Keep the machine, environment, settings, and workload identical for comparisons. Repeat enough to assess variability and retain every run. Investigate material increases in CPU use or memory before claiming no regression.

Retain GNU time output, the command, input or duration, exit status, and diagnostics. Use GNU time's `-o PATH` to separate resource output. Record CPU, OS, build settings, toolchain, and source state, including uncommitted edits. For Linux, also record adapter, driver, display size, and audio setup. Put shareable evidence in distinct paths under `.amp/in/artifacts/`. Keep temporary inputs and raw profiles under `target/`.

CPU seconds can exceed elapsed seconds with multiple threads. Caches remain as found, and machine load affects repeatability. Peak RSS does not measure per-function allocation or total GPU memory. No automatic threshold establishes the absence of a regression. Preserve failing evidence. Never add a performance gate to pre-commit or the correctness check.

## Profile CPU execution separately

For the CLI workload, record a separate user-space sampling pass with DWARF call stacks:

```sh
binary="target/benchmark/$native/release/cli"
perf record -e cpu-clock:u -F 99 --call-graph dwarf,8192 \
	-o target/benchmark/perf.data -- "$binary" \
	< target/benchmark/frames.jsonl > /dev/null
perf report -i target/benchmark/perf.data
```

Linux profiling is allowed only for Linux changes. In that case, substitute the Linux executable and its bounded timeout command. Keep profiling out of ordinary measurement averages.

Perf requires permission to collect performance events. Do not change kernel permissions automatically if it fails. Inspect recording diagnostics and require game samples in `perf report`, not only wrapper-process samples. Add `--stdio` for text output. Inspect function and call-stack attribution. Preserve the matching executable before rebuilding and use distinct paths for separate recordings.

Reject counters marked `<not supported>` or `<not counted>`. They are not zero. A function absent from a short sample may still have substantial cost. The recipe does not attribute GPU work or allocations.
