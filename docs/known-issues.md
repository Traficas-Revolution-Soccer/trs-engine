# Known issues

- The story-demo JSON shape is not yet a versioned external schema.
- `trs demo` advances a deterministic simplified interaction state and does not simulate Rapier, render, or animate; `trs play` is the actual rendered Rapier-enabled scene.
- The runtime cutscene beat is currently a log message over the static scene. No dialogue HUD, camera transition, or character animation is implemented.
- Player movement directly updates the kinematic body's transform; this starter controller does not yet resolve movement against walls or implement gravity/jumping.
- Asset creation/import, Blender orchestration, validation CLI, serialization, and Windows runtime/CI remain future work.
- Desktop rendering requires an available graphics backend and display; headless CI only tests the story simulation.
