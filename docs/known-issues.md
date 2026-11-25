# Known issues

- Scene schema is version 1; backward-compatible migrations are not implemented.
- `trs asset validate` checks the container and glTF version/mesh inventory, not every accessor, extension, texture, or material constraint required by the full glTF specification. Bevy's loader remains the runtime import check.
- The modeler generates colored boxes and a smooth procedural UV sphere; Blender is not installed and its automation path is not implemented. The sample football therefore has no panel pattern or texture.
- NPC animation is a procedural whole-body turn. There are no humanoid skeletons, bone tracks, skinning, facial animation, or retargeting.
- Player movement uses Rapier character-controller collision, gravity, and a single jump. Jump buffering, coyote time, and specialized slope handling are not implemented.
- Camera/dialogue timeline is frame-indexed with linear camera transitions; no audio, VFX, video output, save/load, or NPC navmesh behavior yet.
- Windows runtime, visual regression, and performance benchmarks remain unverified.
