# TRS Engine

**AI-native, CLI-first 3D game engine in Rust and Bevy.** TRS Engine is being built so agents can create, run, validate, and inspect game content through documented commands and APIs instead of depending on manual editor workflows.

> **Status: early MVP.** This repository currently contains a runnable Bevy 3D scene, Rapier colliders, keyboard movement, NPC proximity interaction, a deterministic headless story-slice demo, and a JSON CLI report. Rigging, authored animation assets, Blender automation, GLB import/export workflows, a timeline editor, and a full mission framework are not implemented yet.

## Quick start

Requirements: Rust 1.97.1 or newer, Cargo, and a desktop graphics environment for `play` (Linux X11, Windows, and other Bevy-supported targets as dependencies permit).

```sh
cargo run -p trs-engine -- --help
cargo run -p trs-engine -- demo
cargo run -p trs-engine -- demo --json
cargo run -p trs-engine -- play
```

`demo` runs without opening a window and emits a repeatable story slice: walk to an NPC, start a short dialogue/cutscene beat, return control, and set a football-field objective. `--json` returns structured data for agents. `play` opens a real-time 3D scene; use **W/A/S/D** to approach the orange NPC. The scene uses Rapier fixed colliders for the ground, player, NPC, and street props.

## CLI contract

| Command | Behavior |
| --- | --- |
| `trs --help` | Show commands and usage |
| `trs demo` | Print the deterministic headless vertical slice |
| `trs demo --json` | Print `DemoReport` JSON (`scene`, `systems`, `events`, `slice_passed`) |
| `trs play` | Open the interactive Bevy/Rapier 3D scene |

Demo JSON is a versioned-by-release Rust struct, not a stable schema yet. CLI errors use Clap's standard diagnostics and exit codes. The headless demo exercises scripted gameplay state transitions; it does not render frames or advance the Rapier simulation. `play` is the rendered, physics-enabled runtime.

## Development

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p trs-engine -- demo --json
```

See [AGENTS.md](AGENTS.md) for machine-oriented project guidance, [docs/architecture.md](docs/architecture.md) for current boundaries and compatibility choices, and [docs/known-issues.md](docs/known-issues.md) for the explicit MVP gaps. The project is MIT-licensed. No third-party game assets are included.
