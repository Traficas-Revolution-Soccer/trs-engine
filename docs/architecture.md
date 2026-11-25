# Architecture (MVP)

## Runtime boundaries

The workspace currently has one package, `trs-engine`. Its library contains deterministic story-slice state progression that can run and be tested without a GPU or window. The binary exposes that as `trs demo`, with Serde JSON output for agents. The `play` path builds a Bevy 0.20.1 application, creates the neighborhood scene procedurally, and installs the Rapier 0.37.0 physics plugin with fixed and kinematic colliders.

The split is intentionally small: reusable game state stays independent of Bevy's renderer; the binary owns CLI, user input, scene setup, and engine integration. A workspace is in place, but additional crates will be introduced only when a real boundary needs independent ownership.

## Compatibility decision

Crates.io stable releases checked on 2026-11-24: Bevy 0.20.1 requires Rust 1.97.1; Rapier 0.37.0 declares Bevy 0.20 in its manifest. The `3d` + `x11` Bevy feature set enables a desktop 3D runtime without enabling Bevy's unrelated default audio and 2D bundles. Rapier's debug-render and picking integrations are disabled; 3D rigid bodies and colliders remain enabled.

## Current contracts

- `run_demo()` returns a deterministic `DemoReport`; `trs demo --json` serializes it to JSON.
- The interactive runtime moves the player with WASD and starts the NPC interaction once the player is within 1.8 world units.
- Generated meshes and materials are original simple primitives; no external game content is bundled.

## Deliberately deferred

The MVP does not yet import GLB, invoke Blender, implement a rig/animation pipeline, render cutscene timelines or dialogue UI, persist scenes, export frames, or implement agent editing APIs. The headless demo simulates story progression; it is not the Rapier runtime or a rendered capture. See `known-issues.md`.
