# Known issues

- Scene schema is version 1; backward-compatible migrations are not implemented.
- `trs asset validate` checks the container and glTF version/mesh inventory, not every accessor, extension, texture, or material constraint required by the full glTF specification. Bevy's loader remains the runtime import check.
- The modeler currently creates flat-shaded cuboids only. Blender is unavailable here; no Blender automation has been verified.
- NPC animation is a procedural whole-body turn. There are no humanoid skeletons, bone tracks, skinning, facial animation, or retargeting.
- The kinematic WASD controller clamps to world bounds but does not yet resolve wall collisions or implement gravity/jumping.
- Camera/dialogue timeline is frame-indexed with linear camera transitions; no audio, VFX, video output, save/load, or NPC navmesh behavior yet.
- Windows runtime, visual regression, and performance benchmarks remain unverified.
