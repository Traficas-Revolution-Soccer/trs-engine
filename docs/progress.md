# Progress

## Delivered

- Clap CLI for playback, deterministic demo, scene validation, asset inspection, and procedural model generation.
- Versioned JSON neighborhood scene with validated transforms, bounds, props, objective, dialogue cues, and camera timeline.
- Shared story state machine: range-gated interaction, frame-stepped cutscene dialogue, mission activation, and destination completion.
- Bevy 3D/Bevy UI runtime with Rapier colliders and kinematic character control, gravity/jumping, scene/GLB loading, keyboard movement with collision sliding and world bounds, cinematic camera interpolation, dialogue HUD, and procedural NPC gesture.
- CLI generates validated GLB meshes for boxes and UV spheres from JSON specifications.
- Interactive runtime loads and saves versioned mission/player state bound to a scene, with validated startup load and F5/F9 controls.
- Agent instructions, architecture/progress/known-issue docs, and CI smoke commands.

## Remaining roadmap

Blender orchestration; humanoid rigs/skinning, bone-level animation and retargeting; general mesh editing, UV/material/texture workflows; full glTF conformance and round-trip tests; navigation; automatic checkpoints; rich cutscene authoring, audio/VFX; frame export and visual regression; Windows runtime verification and performance benchmarks.
