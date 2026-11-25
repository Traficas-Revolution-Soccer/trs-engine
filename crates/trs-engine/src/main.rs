use bevy::{prelude::*, window::WindowResolution};
use bevy_rapier3d::prelude::*;
use clap::{Parser, Subcommand};
use trs_engine::run_demo;

#[derive(Parser)]
#[command(
    name = "trs",
    version,
    about = "AI-native 3D game engine",
    long_about = "CLI-first Rust game engine. Run `trs demo --json` for a deterministic vertical slice or `trs play` for its interactive 3D scene."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the complete scripted story-gameplay vertical slice without a display.
    Demo {
        /// Print the structured agent-readable report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Open the interactive 3D scene (WASD to move; approach the NPC to interact).
    Play,
}

fn main() {
    match Cli::parse().command {
        Command::Demo { json } => {
            let report = run_demo();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("demo report serializes")
                );
            } else {
                println!("Scene: {}", report.scene);
                for event in report.events {
                    println!("- {event}");
                }
                println!("Story slice passed: {}", report.slice_passed);
            }
        }
        Command::Play => play(),
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Npc;

#[derive(Resource, Default)]
struct StoryState {
    interacted: bool,
}

fn play() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "TRS Engine — Bairro da Bola".into(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .insert_resource(StoryState::default())
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (move_player, interact))
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(24.0, 24.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.42, 0.22))),
        Transform::default(),
        RigidBody::Fixed,
        Collider::cuboid(12.0, 0.1, 12.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.8, 0.8))),
        MeshMaterial3d(materials.add(Color::srgb(0.15, 0.38, 0.9))),
        Transform::from_xyz(-2.0, 0.9, 0.0),
        Player,
        RigidBody::KinematicPositionBased,
        Collider::capsule_y(0.8, 0.35),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.9, 1.7, 0.7))),
        MeshMaterial3d(materials.add(Color::srgb(0.9, 0.35, 0.12))),
        Transform::from_xyz(2.0, 0.85, 0.0),
        Npc,
        RigidBody::Fixed,
        Collider::capsule_y(0.75, 0.35),
    ));
    for x in [-5.0, 5.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.2, 3.0, 1.2))),
            MeshMaterial3d(materials.add(Color::srgb(0.7, 0.55, 0.37))),
            Transform::from_xyz(x, 1.5, -4.0),
            RigidBody::Fixed,
            Collider::cuboid(0.6, 1.5, 0.6),
        ));
    }
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        PointLight {
            intensity: 2_500_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 8.0, 4.0),
    ));
}

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player: Query<&mut Transform, With<Player>>,
    state: Res<StoryState>,
) {
    if state.interacted {
        return;
    }
    let mut direction = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        direction.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if let Ok(mut transform) = player.single_mut() {
        transform.translation += direction.normalize_or_zero() * 3.0 * time.delta_secs();
    }
}

fn interact(
    mut state: ResMut<StoryState>,
    player: Query<&Transform, With<Player>>,
    npc: Query<&Transform, With<Npc>>,
) {
    if state.interacted {
        return;
    }
    let (Ok(player), Ok(npc)) = (player.single(), npc.single()) else {
        return;
    };
    if player.translation.distance(npc.translation) < 1.8 {
        state.interacted = true;
        info!(
            "Cutscene: NPC says ‘Vamos jogar?’ Camera shot: two-shot. Mission: Encontre o campo de futebol."
        );
    }
}
