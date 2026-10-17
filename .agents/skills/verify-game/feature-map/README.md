# gamz verification feature map

Use the [verification skill](../SKILL.md) for launch, health checks, cleanup, and scope selection. Game-layer changes use only CLI paths. Run each platform path only when that specific platform is altered, including during maintenance audits. The entries below give feature-specific inputs, expected outputs, and coverage limits.

| Feature | Entry | Verification path |
| --- | --- | --- |
| Initialization, frames, and pixels | [Initialization and rendering](launch-and-render.md) | CLI for frames, Linux for pixels. |
| Controller values, time, and keyboard mapping | [Controller input](controller-input.md) | CLI for game input, Linux for keyboard integration. |
| Samples, playback, and underruns | [Audio](audio.md) | CLI for samples, Linux for playback. |
| EOF, errors, Escape, and window close | [Exit behavior](exit.md) | CLI for protocol errors, Linux for window lifecycle. |
| Benchmark correctness, CPU use, memory, and profiling | [Benchmarking and profiling](performance.md) | Example CLI with GNU time and optional perf. Linux measurements only for Linux changes. |
