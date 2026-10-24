# Engine verification map

| Boundary | Recipe | Evidence |
| --- | --- | --- |
| Initialization, frames, controller values, audio, EOF, and failures | [CLI contract](cli.md) | Complete production CLI inputs and outputs. |
| Game and engine CPU cost | [CLI measurements](performance.md) | A fixed workload with independently checked results. |
| Presentation, OS events, playback, and host lifecycle | [Platform checks](platforms.md) | Only the affected platform's behavior, never game correctness. |

`platform-api::Game` defines initialization, frame updates, and audio updates. Follow its current caller to choose a task-specific CLI workload. The CLI cannot inject render commands or force arbitrary engine paths.

Existing `game-core` tests cover parts of asset validation, math, and entity storage. `game-memory` tests and Miri cover arena behavior. Run `cargo test -p game-core -p game-memory --locked` or the full pre-commit hook. These checks do not prove every utility operation or game integration.

Only claim coverage for file reads, texture uploads, render commands, and utility paths actually exercised. CLI serialization does not prove GPU pixels, OS event mapping, or device playback. Source-only findings and out-of-scope platform paths stay explicit in maintenance reports.
