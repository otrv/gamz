# 3D benchmark milestones

Replace Breakout with a small 3D scene, then build toward the animated-character
benchmark in [Gabriel Dechichi's post](https://x.com/gdechichi/status/1935741585368793221).
The initial target is native Linux. Browser delivery is a later decision, and
the reported 14x speedup is not an acceptance requirement.

## How we will work

Milestones describe required outcomes, not implementation plans. Milestone 1 is
the agreed first scope and is in progress. Later milestones are unimplemented.

Plan only the next milestone, using what we learned from the previous one.
At that point, choose focused resources, discuss design decisions and tradeoffs,
and agree on small implementation steps. Later requirements may change with
what we learn. Interfaces, dependencies, and implementation techniques remain open.

Every implementation step must update the runnable benchmark with visible
behavior or an inspectable measurement. After each step, demonstrate and verify
the result, explain the concepts, and record decisions and open questions.
Pause for discussion before proceeding. Follow the repository's existing
approval and verification rules in [AGENTS.md](AGENTS.md).

## 1. A basic 3D scene

- Replace Breakout with a floor, several colored cubes, and a box-shaped player.
- Allow WASD movement on the floor, with a camera following at a fixed offset
  behind and above the player.
- Show perspective and correct depth occlusion, including when draw order changes.
- Keep the existing HUD visible over the scene and preserve correct rendering
  when the window resizes.

Collision, gravity, jumping, camera orbit, lighting, and animation are outside
this milestone. The player can pass through cubes.

The agreed coordinate convention matches Unity's world axes: left-handed,
+Y up, +Z forward. Keep the 1280×720 letterboxed canvas. Use generic indexed
triangles, with opaque 3D rendered before the existing 2D HUD.

The first learning step replaces Breakout with a perspective colored triangle.
Start (Space on Linux) switches between two distances to demonstrate perspective
without changing the mesh or HUD size. Floor geometry, cubes, depth occlusion,
player movement, and the following camera remain to be implemented.

## 2. A repeatable rendering baseline

- Run the same scene and camera movement repeatedly at selected object counts.
- Report clearly defined timings and workload counts with enough environment
  information to compare runs.
- Distinguish measured work from startup costs and frame pacing.

## 3. Reusable mesh resources

- Render multiple objects from shared geometry without repeatedly uploading
  unchanged mesh data.
- Keep resource use bounded and support releasing and recreating the scene's meshes.
- Measure the effect on the existing benchmark.

## 4. A visible animated skeleton

- Display a small hierarchy of connected joints whose child motion follows its parents.
- Play a short looping animation and allow inspection of individual poses.

## 5. A deforming mesh

- Deform a simple mesh using the animated skeleton, including vertices influenced
  by more than one joint.
- Preserve the original shape in the bind pose, the skeleton's reference pose.
- Make the original and animated shapes easy to compare in the benchmark.

## 6. An imported character

- Display one appropriately licensed character asset at the expected scale and orientation.
- Preserve its reference pose and make its skeleton alignment inspectable.
- Report the geometry and skeleton counts used by the benchmark.

## 7. An animated character

- Play one imported running animation on the character.
- Match the source animation at selected poses and across the loop boundary.
- Allow playback to pause for inspection.

## 8. An animated crowd

- Display an adjustable grid of characters with independent animation phases.
- Keep character counts and memory use within explicit budgets.
- Report evaluated and drawn counts, with measurements that distinguish animation
  work from rendering work.

## 9. A measured performance improvement

- Address a bottleneck identified in the crowd benchmark.
- Preserve the workload and animation output while measuring the change's effect.
- Keep enough evidence to compare the result with the baseline.

## 10. The reference benchmark workload

- Recreate the reference's running-character grid and controls for adding or removing characters.
- Make runs reproducible and record differences in assets, rendering, and workload.
- Report performance with its hardware and settings, without claiming equivalence
  between native and browser results.

Browser support and direct engine comparisons require separate planning if we
choose to pursue them. Third-person gameplay is outside these benchmark milestones.

## Learning resources

Use these as starting points, then select specific reading when planning each
milestone. They do not prescribe an API, asset format, or implementation.

- For the first scene, read [Coordinate systems](https://learnopengl.com/Getting-started/Coordinate-Systems)
  and [Depth testing](https://learnopengl.com/Advanced-OpenGL/Depth-testing).
  OpenGL API details and depth conventions differ from `wgpu`.
- For measurement, read [WebGPU timing](https://webgpufundamentals.org/webgpu/lessons/webgpu-timing.html).
- For animation, consult the Khronos tutorials on
  [joint hierarchies](https://github.khronos.org/glTF-Tutorials/gltfTutorial/gltfTutorial_004_ScenesNodes.html),
  [animation tracks](https://github.khronos.org/glTF-Tutorials/gltfTutorial/gltfTutorial_007_Animations.html),
  and [skinning](https://github.khronos.org/glTF-Tutorials/gltfTutorial/gltfTutorial_020_Skins.html).
- For the target workload, inspect the pinned
  [reference Unity project](https://github.com/gabrieldechichi/unity_webglperftest/tree/17abd749d1b786017b9b7afacd3576735f6d8cd3).
