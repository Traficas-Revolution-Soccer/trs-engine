# Architecture

## Runtime

The Rust workspace has one package (`trs-engine`). `src/lib.rs` owns scene-schema validation and a deterministic gameplay state machine shared by CLI demo tests and the interactive runtime. `src/main.rs` adapts that core to Clap and Bevy 0.20.1. Scene JSON provides player/NPC transforms, prop layout, field objective, frame-keyed dialogue, and camera shots; the Bevy runtime loads the same scene and its referenced GLB assets.

Bevy provides desktop rendering and glTF scene loading. Rapier 0.37.0 provides fixed and kinematic colliders. The small controller and gameplay state remain independently testable without a GPU. Generated sample geometry is original.

## Agent workflows

- `trs scene validate FILE --json`: parse and validate versioned scene data, finite/positive dimensions, world bounds, timeline ordering, and safe relative GLB references.
- `trs model create --spec FILE --output FILE --json`: create a low-poly cuboid GLB from a JSON shape/material specification.
- `trs asset validate FILE --json`: check glTF 2.0 JSON or the GLB header/JSON chunk and report mesh count.
- `trs demo --json`: deterministically advance the shared interaction/cutscene/mission state and report each emitted event.

## Compatibility

Crates.io stable releases checked on 2026-11-24: Bevy 0.20.1 requires Rust 1.97.1; Rapier 0.37.0 declares Bevy 0.20. Bevy features include `3d`, `ui`, and `x11`; unrelated default audio support stays disabled.

## Boundaries

Model generation is box-only; the asset inspector does not replace a full glTF conformance validator. The actor's cutscene gesture is a procedural body rotation, not skeletal animation. Blender is not installed in the execution environment; no unverified Blender bridge is claimed. Rigging, arbitrary mesh editing, retargeting, save/load, navigation, cinematic audio/VFX, render capture, regression images, and Windows validation remain future work.
