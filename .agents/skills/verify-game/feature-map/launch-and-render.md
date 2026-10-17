# Initialization, frame output, and rendering

## Check initialization and frames through the CLI

Run `launch`, `doctor`, then `drive`. Expect initialization with `uploads: []`. Each visual input returns a frame with canvas `[1280,720]`, clear `[0,0,0,255]`, and `commands: []`. No frames arrive without input. Inspect JSONL and status evidence.

Real asset reads use the same file service as Linux, relative to the working directory. The default game does not load assets or return textures, so its baseline cannot exercise those paths. The [benchmark example](performance.md) exercises embedded assets and texture uploads, but not external file reads. When a game loads external assets, add a real asset-loading run rather than claiming the empty baseline covers it.

## Check rendering only when Linux is altered

Run `.agents/skills/verify-game/scripts/verify-game linux`. The platform requests a 1280×720 logical inner window titled `gamz`, initializes WGPU, and continuously presents the black frame. The helper uses a private Xvfb display; software Vulkan may supply the adapter in an orb.

Inspect `linux-before.png` and `linux-after.png` with `view_media`. Expect a black canvas in both. Check `linux.log` for the adapter and `commands=0 quads=0` statistics, then require exit status zero. This verifies empty-frame presentation and process survival, not gameplay, scaling, clipping, or texture creation. Use the [benchmark rendering recipe](performance.md) for a populated scene. CLI commands alone do not prove rendered pixels.
