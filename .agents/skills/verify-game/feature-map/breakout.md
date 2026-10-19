# Verify Breakout and 2D math

Use the production CLI for gameplay checks. Keep complete inputs and outputs for each run.
Do not run Linux checks unless the Linux platform changes.

## Build and check the initial frame

```sh
cargo build --release --locked -p platform --no-default-features \
	--features cli,game-entry/example-breakout --bin cli --target-dir target/verify-breakout
printf '%s\n' '{"dt":{"secs":0,"nanos":0},"controller":{}}' \
	| target/verify-breakout/release/cli
```

Require a flushed `{"uploads":[]}` before sending input. The initial frame contains 32 bricks,
three life markers, and a ball centered at `(640,644)`. The paddle has center x=640 and top y=650.
Identify bricks by their rectangle size `[136,28]`, the paddle by `[144,14]`, and the ball by `[12,12]`.
The ready state includes an aim line. Repeated zero-dt frames must match exactly.

## Drive controls and collisions

- Hold `move_left` or `move_right`. A 50 ms frame moves the paddle 35 units.
  Its center stops at x=96 and x=1184. Holding both buttons cancels movement.
  Before a serve, the ball follows the paddle.
- Press `start` with `ended: "down"` and `half_transitions: 1` to serve.
  A held button with zero transitions must not serve. A press and release within one frame,
  represented by `ended: "up"` and `half_transitions: 2`, must serve.
- From a fresh centered serve at zero dt, advance 10 ms without buttons.
  Compare the ball center with `(640 + 4.2*sin(0.35), 644 - 4.2*cos(0.35))` within 0.0002 units.
  Compare fresh processes given 50 ms and the maximum duration. Their frames must match.
- Follow the ball with paddle input until all bricks disappear. Check brick rectangles after
  every frame. Removed bricks must not reappear, including when fewer than four remain.
  Check paddle and wall reflections and require the green terminal bar at `[480,352,320,16]`.

Move the paddle away to miss each of three balls. Require one fewer life marker after each miss.
The first two misses return to ready. The third removes the ball and shows a red terminal bar.
Require stationary terminal frames. Press `start` after either terminal state and compare the
result with the initial frame. Restart must restore all bricks and three lives without serving.
Interleave audio requests and require silence without changing the next zero-dt frame.

## Check repeatability and numeric behavior

Replay each saved input sequence in a fresh process with the same executable and floating-point
environment. Compare complete outputs byte for byte. Cross-build and cross-architecture equality
are not part of the contract.

Run `cargo test -p game-core --locked` for independent SIMD lanes, extrapolation, rotations,
and normalization of zero, non-finite, very large, and subnormal vectors.
For performance comparisons, use a recorded active playthrough with the [CLI measurement recipe](performance.md).
The existing particle benchmark does not execute the new math operations.
