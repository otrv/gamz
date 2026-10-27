# Game-core capability checklist

This checklist supports scope review for small to medium-sized 2D games. The
representative starter games are a platformer, a metroidvania, and a top-down
action game. They are reference workloads, not games approved for implementation.

Memory, capacity, and work budgets must be predictable and controllable. Exact
limits remain undecided. Text is scoped to English for now, with flexible game
UI and debug text rather than only fixed HUD labels.

Every utility area below is a separate future decision loop with the user,
likely handled by a separate agent. This document does not start those loops,
assign agents, choose designs, decide whether to implement candidates, or order
work. Existing utilities need auditing and completeness decisions too.

All boxes start unchecked. A checked box means the scope decision is recorded,
not that implementation is complete. Each decision records the concrete need,
the owning crate, and whether to retain, complete, extend, add, defer, or exclude
the capability. No candidate is an implementation commitment.

## Repository basis and ownership

The source inspection covered every utility and its tests in `game-core`,
Breakout's use of those utilities, and adjacent crate contracts. The inventory
describes source, not fresh runtime verification or measured game-size support.

| Existing utility | Initial suitability judgment |
| --- | --- |
| [`EntityStorage` and `EntityId`](../crates/game-core/src/entity.rs) | Useful fixed-capacity pools with stale-handle rejection within a store. Handles have no store identity, values require `Copy`, and iteration scans capacity. Suitability depends on entity counts and access patterns. |
| [`Pack`](../crates/game-core/src/asset.rs) | Bounded, borrowed whole-pack reading with structural and exact-length validation. It is not a loader, cache, or content-integrity checker. |
| [`PackCatalog`](../crates/game-core/src/asset.rs) | Useful metadata-only range lookup. Metadata cannot establish payload presence or consistency across separate file reads. |
| [`read_rgba`](../crates/game-core/src/asset/rgba.rs) | A narrow borrowed-image reader with nonzero dimensions and exact payload-length validation. Other image formats are separate scope questions. |
| [`Font`](../crates/game-core/src/font.rs) | Useful printable-ASCII bitmap text with kerning, newlines, measurement, and caller-supplied glyph output. Flexible English UI and debug text need a completeness review beyond fixed labels. |
| [`render::push`](../crates/game-core/src/render.rs) | Bounded single-command insertion with explicit exhaustion. Sprite preparation, ordering, batching, and GPU validation are separate responsibilities. |

The [coding standards](../CODING_STANDARDS.md#layer-ownership) remain authoritative.
Game-core owns game-independent mechanisms extracted from concrete use. Games
compose independent utilities and own rules, state layout, and content choices.
Mathematical operations belong in `math`, typed byte storage belongs in `memory`,
and boundary declarations belong in `platform-api`. Hosts own devices and I/O.
The renderer owns GPU execution. Asset conversion belongs in host build tooling.
Listing an area here does not move its ownership or approve policy changes.

## Existing entity storage

- [ ] Audit handle behavior across removal, clear, separate same-type stores, reconstructed stores, and scene lifetimes.
- [ ] Decide whether current stale-handle guarantees are complete for the starter games.
- [ ] Evaluate fixed inline capacity, the `Copy` requirement, initialization, stack use, and sparse iteration cost.
- [ ] Decide whether caller-supplied storage, dense iteration, or other storage capabilities are needed without choosing a representation yet.
- [ ] Audit full and zero capacity, repeated removal, and generation-exhaustion behavior.
- [ ] Evaluate multiple-entity mutation and insertion or removal during traversal.
- [ ] Decide whether independent pools suffice or concrete games require component storage and queries.
- [ ] Decide whether live handles need separate durable identifiers for persistence and replay.

## Existing whole-pack reader

- [ ] Audit versions, count and byte budgets, arithmetic bounds, malformed metadata, truncation, and trailing-byte rejection.
- [ ] Audit ID lookup, exact name lookup, empty payloads, and missing assets.
- [ ] Decide how pack-local numeric IDs and names behave across rebuilt content.
- [ ] Evaluate retained-byte lifetime, parsing cost, lookup cost, and memory use for the starter games.
- [ ] Decide whether structural validation is complete for intended content or whether integrity and compatibility checks are needed.
- [ ] Record which loading, caching, and residency requirements belong outside this reader.

## Existing pack catalog

- [ ] Audit header sizing, metadata-only parsing, range arithmetic, and declared pack budgets.
- [ ] Confirm which validation guarantees differ from whole-pack validation.
- [ ] Evaluate catalog-only loading against whole-pack loading without selecting one for every game.
- [ ] Evaluate short reads, chunk boundaries, queue exhaustion, cancellation, late completions, and files changing between reads.
- [ ] Decide how games retain metadata and payload bytes without persisting borrowed views.
- [ ] Decide whether any reusable loading mechanism is missing while keeping game scheduling and eviction choices separate.

## Existing RGBA reader

- [ ] Audit dimensions, exact payload lengths, overflow, truncation, and borrowed-output lifetime.
- [ ] Confirm pixel order, color-space, and alpha expectations across content production and rendering.
- [ ] Evaluate decoded-image memory use separately from structural validity.
- [ ] Decide whether this reader is complete for the intended offline conversion workflow.
- [ ] Evaluate additional runtime image readers as separate candidates rather than assuming PNG decoding is required.

## Existing bitmap font

- [ ] Audit font and atlas validation, glyph bounds, kerning records, numeric limits, and fixed storage cost.
- [ ] Audit measurement and preparation for empty text, spaces, newlines, unsupported characters, and asymmetric glyph offsets.
- [ ] Audit exact output capacity, exhaustion, and partially written output on failure.
- [ ] Decide whether advance metrics suffice or whether visual bounds, baselines, and richer line metrics are needed.
- [ ] Evaluate wrapping, alignment, clipping, truncation, tabs, line spacing, and multiple fonts for English UI and debug text.
- [ ] Decide which English characters and symbols need coverage beyond printable ASCII, including useful debug symbols.
- [ ] Evaluate dynamic formatted text rather than only predefined labels.
- [ ] Evaluate font-production tooling separately from runtime layout.
- [ ] Record non-English localization, shaping, and bidirectional text as outside the current text scope, without choosing a future design.

## Existing render-command insertion

- [ ] Audit command-length invariants, exact capacity, exhaustion, and preservation of accepted commands.
- [ ] Decide whether single-command insertion is complete for its narrow responsibility.
- [ ] Evaluate future multi-command output needs and partial-failure semantics without selecting a builder design.
- [ ] Evaluate command and quad budgets for representative scenes.
- [ ] Keep command construction separate from renderer validation, texture residency, and GPU resource policy.

## Bounded collections and text formatting

- [ ] Evaluate bounded lists, ring buffers, queues, bitsets, key lookup, and sparse indexing against concrete game needs.
- [ ] Compare each candidate with existing arrays, slices, and entity storage before deciding whether to add it.
- [ ] Evaluate batch operations, removal order, iteration order, and exhausted-output behavior.
- [ ] Decide persistent, frame-scratch, and borrowed-view lifetimes for accepted mechanisms.
- [ ] Evaluate capacity calculations and high-water observations without making game-core own the game's storage layout.
- [ ] Evaluate bounded formatting through `core::fmt::Write` for UI, scores, and debug text.

## Time and simulation clocks

- [ ] Evaluate fixed-step accumulation, catch-up bounds, and excess elapsed-time behavior.
- [ ] Evaluate timers, cooldowns, periodic triggers, pause, time scaling, and suspension.
- [ ] Evaluate zero elapsed time, long frames, and updates crossing several deadlines.
- [ ] Evaluate render interpolation needs while keeping game scheduling and state ownership explicit.

## Logical input interpretation

- [ ] Evaluate press, release, held, repeat, and multiple-transition interpretation of existing logical buttons.
- [ ] Decide how input edges and continuous values transfer to zero, one, or several simulation ticks.
- [ ] Evaluate buffered actions, hold thresholds, and double-tap recognition for the starter games.
- [ ] Evaluate pointing, scrolling, text, UI focus, and gameplay input consumption needs.
- [ ] Separate game action interpretation from host-owned physical bindings, device mapping, and controller discovery.

## Randomness and sampling

- [ ] Evaluate seeded random sequences, explicit state, independent streams, and saved-state requirements.
- [ ] Evaluate unbiased bounded integers, weighted selection, and spatial sampling.
- [ ] Decide required repeatability across runs, builds, and architectures without selecting a generator yet.
- [ ] Separate host entropy from deterministic game-side mechanisms.

## Geometry, transforms, and cameras

- [ ] Evaluate missing rectangle, scalar, vector, angle, curve, and transform operations under `math` ownership.
- [ ] Decide required coordinate spaces, units, edge rules, and angle conventions.
- [ ] Evaluate local and world transforms, inverse transforms, pivots, and nonuniform scale.
- [ ] Evaluate whether transform hierarchies are needed, including depth bounds, cycles, and derived-data invalidation.
- [ ] Evaluate camera conversion, inverse pointing conversion, visible bounds, follow behavior, dead zones, world limits, and shake.
- [ ] Separate game-side pixel snapping from host viewport, letterboxing, and framebuffer conversion.
- [ ] Identify capabilities that need changes beyond the current origin-and-zoom camera command.

## Collision and movement

- [ ] Evaluate overlap, ray, segment, and swept queries for the shapes required by the starter games.
- [ ] Audit potential extraction from Breakout's game-local collision code without assuming its rules generalize.
- [ ] Decide touching-edge, initial-overlap, zero-motion, corner-hit, and equal-time-hit semantics.
- [ ] Evaluate finite-input domains, tolerances, degenerate shapes, and large-coordinate behavior.
- [ ] Evaluate broad-phase queries, filtering, self-exclusion, and bounded candidate output.
- [ ] Evaluate trigger enter, stay, and exit tracking across movement and despawn.
- [ ] Evaluate sweep-and-slide, slopes, one-way platforms, and moving-platform needs while distinguishing mechanisms from game rules.
- [ ] Evaluate rigid-body integration, contact resolution, friction, restitution, and joints without deciding whether to implement a physics system.
- [ ] Evaluate crowded scenes, high speed, repeated impacts, and exhausted work budgets.

## Sprites and drawing preparation

- [ ] Evaluate atlas regions, pivots, trimming offsets, flipping, rotation, tint, and sampling needs.
- [ ] Identify requirements that current render commands cannot express rather than assuming support.
- [ ] Evaluate caller-buffered quad preparation, visibility filtering, stable layering, and transparency-safe batching.
- [ ] Evaluate clip composition, camera changes, and state restoration requirements.
- [ ] Evaluate nine-slice panels, repeated backgrounds, and required shape generation.
- [ ] Separate these candidates from render targets, custom shaders, lighting, blend modes, and post-processing owned by the renderer and its contract.

## Animation and value playback

- [ ] Evaluate sprite clips, playback rates, looping, reverse playback, ping-pong, pause, seek, and completion.
- [ ] Decide skipped-frame and multiple-marker behavior across long updates.
- [ ] Evaluate bounded animation events as game-consumed data.
- [ ] Evaluate value tracks and tween playback while keeping mathematical curve evaluation in `math`.
- [ ] Evaluate competing property writes and cancellation when an entity or scene disappears.
- [ ] Include skeletal animation and animation graphs as undecided candidates, not assumed requirements.

## Particles and procedural effects

- [ ] Evaluate particle state, spawn budgets, expiry, and exhaustion behavior.
- [ ] Evaluate trails, screen transitions, and other procedural effects against starter-game needs.
- [ ] Evaluate independent simulation and output data that games can compose with randomness and rendering.
- [ ] Decide which effects justify reusable mechanisms and which remain game-specific.

## Tiles, grids, and navigation

- [ ] Evaluate bounded grids, negative coordinates, world-to-cell conversion, and out-of-range behavior.
- [ ] Evaluate orthogonal, isometric, hexagonal, and chunked maps without selecting supported forms yet.
- [ ] Evaluate visible-cell queries, metadata, collision flags, dynamic occupancy, and map decoding.
- [ ] Evaluate flood fill, line of sight, pathfinding, navigation meshes, steering, and crowd simulation as distinct candidates.
- [ ] Evaluate path costs, tie-breaking, unreachable goals, scratch capacity, and bounded or incremental search work.
- [ ] Keep editor-format conversion and content generation separate from runtime utilities.

## Flexible English UI and debug text

- [ ] Evaluate independent layout, hit testing, focus navigation, scrolling, panels, buttons, and lists.
- [ ] Evaluate bounded English text editing, cursor movement, selection, and validation.
- [ ] Identify host-contract gaps for pointer buttons, clipboard operations, text composition, or richer navigation.
- [ ] Evaluate debug labels, multiline logs, overlays, and changing numeric values with controllable text and draw budgets.
- [ ] Decide whether widget mechanisms or a UI framework offer concrete value beyond game-owned composition.
- [ ] Keep menu flow, screen selection, and visual style as game decisions unless concrete reuse justifies extraction.

## Asset formats and resource lifecycle mechanisms

- [ ] Evaluate readers for sprite, animation, tile, font, and audio data independently of pack lookup.
- [ ] Evaluate offline conversion versus runtime decoding without selecting formats or dependencies yet.
- [ ] Evaluate reusable loading-state needs separately from game-owned scheduling, caching, and eviction policy.
- [ ] Evaluate streaming, hot replacement, compression, and modded content as undecided candidates.
- [ ] Evaluate compatibility and corruption-detection needs for external content.
- [ ] Keep filesystem access, file watching, texture uploads, and device limits with their existing owners.

## Audio processing

- [ ] Evaluate sampled effects, music, synthesis, mixing, and bounded voice storage.
- [ ] Evaluate start, stop, looping, priorities, voice exhaustion, gain, panning, fades, and clipping behavior.
- [ ] Evaluate sample-rate conversion, playback-rate changes, cursor progression, and arbitrary output block sizes.
- [ ] Evaluate audio readers separately from loading and mixing.
- [ ] Evaluate frame-to-audio requests with explicit ownership and scheduling assumptions.
- [ ] Include streaming music, spatial attenuation, and audio effects as separate undecided capabilities.
- [ ] Keep device negotiation, buffering, underruns, and physical playback outside game-core.

## Persistence, replay, and game flow

- [ ] Evaluate bounded binary encoding and decoding for saves, settings, and replay data.
- [ ] Evaluate versions, endianness, lengths, malformed data, and migration requirements without choosing schemas.
- [ ] Separate live handles from durable identity and reconstructable entity relationships.
- [ ] Evaluate tick input, random state, checksums, and determinism requirements for replay.
- [ ] Evaluate scene transitions and state-machine mechanisms without moving game selection or memory layout into game-core.
- [ ] Include dialogue, behavior trees, scripting, quests, inventories, and combat mechanisms as undecided candidates rather than presumed shared systems.
- [ ] Keep save schemas, progression, atomic file operations, and game-specific flow with their appropriate owners.

## Adjacent capabilities and ownership decisions

- [ ] Evaluate ECS, scene frameworks, and other broad frameworks against smaller mechanisms without deciding on inclusion or design.
- [ ] Record platform, web, mobile, local multiplayer, touch, and accessibility requirements separately from game-core utility decisions.
- [ ] Evaluate networking requirements while separating platform transport, game protocol, rollback, lockstep, and reusable data processing.
- [ ] Evaluate editors, importers, debuggers, profilers, and content validation as tooling responsibilities.
- [ ] Evaluate screenshots, GPU timing, resource inspection, and render diagnostics under host and renderer ownership.
- [ ] Evaluate whether concrete workloads justify background jobs, with scheduling and synchronization owned explicitly.

## Questions shared by every future decision loop

- [ ] Record the starter-game need and whether an existing utility is complete, needs work, or is unsuitable for that need.
- [ ] Decide inclusion separately from design and implementation authorization.
- [ ] Identify ownership and shared guarantees without coupling sibling utility implementations.
- [ ] Require predictable, controllable storage and work budgets. Leave exact limits to the relevant future decision.
- [ ] Evaluate lifetimes, invalidation, exhaustion, numeric domains, and malformed-input behavior.
- [ ] Evaluate complete arena-backed usage before approving an API, under the existing repository rules.
- [ ] Identify public-contract and production CLI evidence needed to establish correctness and normal game use.
- [ ] Identify host or renderer evidence separately where CLI output cannot prove pixels, device behavior, or audible playback.
- [ ] Record approvals required for public interfaces, crates, dependencies, and any policy changes.
- [ ] Record unresolved questions and the user's decision without turning this checklist into an execution plan.
