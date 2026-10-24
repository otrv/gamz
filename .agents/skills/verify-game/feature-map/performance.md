# Measure CLI execution

Use only the CLI for game-layer performance. Skill-only maintenance needs no benchmark. Measure a graphical platform only for changes to its own operation.

Choose a fixed JSONL workload that exercises the changed code, including audio requests when relevant. Check it through the helper first. Keep its inputs, expected observations, assets, memory capacities, and working directory fixed across revisions. An idle or terminal game may no longer execute the intended path.

Build and validate the measured release executable separately:

```sh
cargo build --release --locked -p platform --no-default-features \
	--features cli --bin cli --target-dir target/measure-game
target/measure-game/release/cli < target/requests.jsonl > target/output.jsonl
```

Require zero status and no diagnostics. Check complete release outputs against independent expectations. Preserve each executable and source revision before building the next; never revert others' work to obtain a baseline.

Preselect repetitions and record each run separately:

```sh
/usr/bin/time -v -o target/run-1.time.txt \
	target/measure-game/release/cli < target/requests.jsonl > /dev/null
```

Keep builds, input generation, captures, and concurrent checks outside timing. Retain statuses, stderr, workload hash, executable identity, machine, toolchain, build flags, asset contents, and variability. Use the same asset root and memory flags as correctness verification.

This measures initialization, parsing, game work, serialization, flushes, and writes together. `/dev/null` removes regular-file storage, not serialization or write calls. Peak RSS is not per-function allocation. Short runs may be dominated by startup. Report these limits and unmeasured GPU, playback, or unexecuted paths.

Profile separately if needed. Build with `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, then use `perf record -e cpu-clock:u -F 99 --call-graph dwarf,8192 -- <binary>` with the same input. Retain the matching executable and inspect `perf report --stdio`. Do not change kernel permissions automatically, count failed events as zero, or include profiling runs in timing averages. No performance gate belongs in the correctness baseline.
