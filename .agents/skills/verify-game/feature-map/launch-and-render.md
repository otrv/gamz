# Launching and rendering

## Sub-features

- A 1280 by 720 window titled `gamz` opens.
- The game initializes and continuously presents frames.
- The boilerplate frame is opaque black.

## How to get to it (user POV)

From the repository root, run `cargo run -p platform --features linux --bin linux --locked`. A user sees a window titled `gamz`.

## Driving it with verify-game

Run `.agents/skills/verify-game/scripts/verify-game launch`, then `doctor`, then `drive`. Inspect the before and after PNG files printed by `drive`. Inspect the copied game log for `wgpu adapter:` and a `commands=0 quads=0` frame-stat line.

## Gotchas

The game needs an X11 display and a GPU driver. The helper supplies Xvfb; WGPU may use a software Vulkan driver in an orb. The current black frame is intentional, so a black screenshot proves presentation only, not future gameplay behavior.
