# Initialization, frame output, and rendering

## Game path: CLI

Run `launch`, `doctor`, then `drive`. Expect an initialization record with `uploads: []`, followed by one frame per supplied input: canvas `[1280,720]`, clear `[0,0,0,255]`, and `commands: []`. No frames arrive without input. Inspect JSONL and status evidence; no display or GPU is needed.

Real asset reads use the same file service as Linux, relative to the working directory. The current game does not load assets or return textures, so live game verification cannot yet exercise those paths. When the game gains assets, add a real asset-loading run rather than claiming the empty baseline covers it.

## Rendering path: Linux only when affected

Run `.agents/skills/verify-game/scripts/verify-game linux`. The platform requests a 1280×720 logical inner window titled `gamz`, initializes WGPU, and continuously presents the black frame. The helper uses a private Xvfb display; software Vulkan may supply the adapter in an orb.

Inspect both `linux-before.png` and `linux-after.png` with `view_media`, and the retained `linux.log` for adapter and `commands=0 quads=0` stats. Require successful exit status. This proves pixel presentation and platform survival, not gameplay response. CLI output alone cannot verify rendering, scaling, clipping, GPU resource creation, or driver behavior.
