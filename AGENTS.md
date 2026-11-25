# Agent instructions

## Project shape

- Cargo workspace; executable package: `crates/trs-engine` (`trs` binary and `trs_engine` library).
- `src/lib.rs` owns the deterministic, display-free story-slice simulation and its tests.
- `src/main.rs` owns CLI dispatch and the interactive Bevy/Rapier scene.
- Keep gameplay rules testable without a graphics context. Keep Bevy presentation and input in the runtime layer.
- Do not describe roadmap items as implemented. Document scope and known limits alongside behavior.

## Commands

- `cargo run -p trs-engine -- --help`
- `cargo run -p trs-engine -- demo --json` (headless and CI-safe)
- `cargo run -p trs-engine -- play` (desktop display required; WASD moves the player)
- `cargo test --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Contracts and assets

- CLI output intended for agents must be structured and deterministic where possible.
- Validate observable behavior with tests; do not use placeholder output to claim a subsystem works.
- Only add assets with a clear compatible license; current scene geometry is generated in Rust.
- Bevy `0.20.1` and `bevy_rapier3d` `0.37.0` are a compatible pair. Update them together when changing the Bevy major/minor line.
