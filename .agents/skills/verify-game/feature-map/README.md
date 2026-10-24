# Engine verification map

| Boundary | Recipe | Evidence |
| --- | --- | --- |
| Initialization, frames, controller values, audio, EOF, and failures | [CLI contract](cli.md) | Complete production CLI inputs and outputs. |
| Game and engine CPU cost | [CLI measurements](performance.md) | A fixed workload with independently checked results. |
| Presentation, OS events, playback, and host lifecycle | [Platform checks](platforms.md) | Only the affected platform's behavior, never game correctness. |

Hosts call the `platform_api::Game` trait with `&mut impl Platform<'p>`. The game-facing `platform_api::Platform` trait exposes safe references without backend or polling access. The shared implementation is `platform::Host` in `crates/platform/src/host.rs`. Follow the current game implementation to choose a task-specific CLI workload. The CLI cannot inject render commands or force arbitrary engine paths.

Existing `game-core` tests cover parts of asset validation, math, and entity storage. `game-memory` tests and Miri cover arena behavior. Run `cargo test -p game-core -p game-memory --locked` for unit tests. The full pre-commit hook also runs Miri for `game-memory`. Verify boundary behavior through the production CLI with task-specific workloads. Passing checks do not prove every utility operation or game integration.

Only claim coverage for asynchronous file reads and writes, completion reservations, texture uploads, render commands, and utility paths actually exercised. Use task-specific CLI workloads for integration evidence. CLI serialization does not prove GPU pixels, OS event mapping, or device playback. Source-only findings and out-of-scope platform paths stay explicit in maintenance reports.
