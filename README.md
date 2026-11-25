# TRS Engine

AI-native, CLI-first 3D game engine in Rust and Bevy. Create and validate world scenes, generate original GLB models, run missions, and inspect results through commands and Rust APIs.

## Run

Requirements: Rust 1.97.1 or newer and Cargo. `play` requires a desktop display and graphics backend.

```sh
cargo run -p trs-engine -- --help
cargo run -p trs-engine -- scene validate assets/scenes/bairro-da-bola.json --json
cargo run -p trs-engine -- model create --spec assets/models/football.json --output assets/models/football.glb --json
cargo run -p trs-engine -- asset validate assets/models/football.glb --json
cargo run -p trs-engine -- demo --json
cargo run -p trs-engine -- play --save saves/story.json
```

The `demo` command executes the same deterministic mission progression as the runtime: approach Carlos, trigger a 120-frame dialogue/camera timeline, regain control, reach the soccer field, complete the objective, and return a JSON trace. `play` loads the JSON scene and GLB through Bevy, uses Rapier colliders, and presents the objective/dialogue HUD. **W/A/S/D** moves, **Space** jumps, and **E** starts the NPC interaction. Pass `--save FILE` to load progress on startup; press **F5** to save and **F9** to reload that file during play. Saves are versioned JSON bound to the scene and reject invalid progress or out-of-bounds positions.

## CLI

| Command | Purpose |
| --- | --- |
| `trs demo [--json]` | Run the deterministic full story-mission test |
| `trs play [--scene FILE] [--save FILE]` | Play a scene; optionally load/save mission progress |
| `trs scene validate FILE [--json]` | Validate scene schema, transforms, bounds, dialogue, and camera timeline |
| `trs model create --spec FILE --output FILE [--json]` | Generate a colored box or UV-sphere mesh as a glTF 2.0 binary (`.glb`) |
| `trs asset validate FILE [--json]` | Validate glTF/GLB container header and glTF 2.0 document |

JSON scene format: [assets/scenes/bairro-da-bola.json](assets/scenes/bairro-da-bola.json). Model specification: [assets/models/football.json](assets/models/football.json). Set `shape` to `box` or `sphere` (omitted means `box`); specify dimensions and RGBA color in the same JSON. Scene transforms, prop placement, camera shots, and dialogue cues are edited as data; the runtime loads the same format that `scene validate` checks.

## Scope

Implemented: Bevy 3D renderer and GLB import; Rapier character-controller collision, gravity and jumping; procedural neighborhood with detailed houses, trees, a marked soccer pitch, and stylized player/NPC models; player-facing movement; eased cutscene camera shots and smooth player-follow camera; NPC interaction/gesture, mission progression, deterministic JSON trace, versioned mission save/load, scene/asset validation, and procedural box/UV-sphere GLB generation.

Still incomplete: Blender orchestration, football panel/texturing, humanoid rigging/skinning, skeletal and facial animation/retargeting, general mesh editing and UV/texturing tools, authored audio/VFX tracks, navigation/NPC behavior, automatic checkpointing, video export and visual regression. The generated sphere has no soccer-ball panel pattern, and the procedural gesture animates a primitive, not a rigged character.

## Development

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p trs-engine -- demo --json
```

See [AGENTS.md](AGENTS.md), [docs/architecture.md](docs/architecture.md), [docs/progress.md](docs/progress.md), and [docs/known-issues.md](docs/known-issues.md). No proprietary game assets are included; generated sample geometry is original. MIT-licensed.
