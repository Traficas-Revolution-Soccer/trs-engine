use bevy::{prelude::*, window::WindowResolution};
use bevy_rapier3d::prelude::*;
use clap::{Parser, Subcommand};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
use trs_engine::{
    ActorSpec, AssetReport, GamePhase, GameProgress, ModelSpec, SceneSpec, create_box_glb,
    parse_scene, run_demo, validate_gltf, validate_scene_assets,
};

#[derive(Parser)]
#[command(name = "trs", version, about = "AI-native 3D game engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the complete, deterministic story mission without a display.
    Demo {
        #[arg(long)]
        json: bool,
    },
    /// Open the interactive 3D scene.
    Play {
        #[arg(long, default_value = "assets/scenes/bairro-da-bola.json")]
        scene: PathBuf,
    },
    /// Validate a declarative world scene.
    Scene {
        #[command(subcommand)]
        command: SceneCommand,
    },
    /// Inspect a glTF 2.0 JSON or binary asset.
    Asset {
        #[command(subcommand)]
        command: AssetCommand,
    },
    /// Generate an original primitive model as a GLB file.
    Model {
        #[command(subcommand)]
        command: ModelCommand,
    },
}

#[derive(Subcommand)]
enum SceneCommand {
    Validate {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum AssetCommand {
    Validate {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ModelCommand {
    Create {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match execute(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("TRS_ERROR: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute(command: Command) -> Result<(), String> {
    match command {
        Command::Demo { json } => {
            let report = run_demo()?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
                );
            } else {
                println!("Scene: {}", report.scene);
                for event in &report.events {
                    println!("- {event}");
                }
                println!("Mission phase: {:?}", report.final_phase);
                println!("Story slice passed: {}", report.slice_passed);
            }
        }
        Command::Play { scene } => play(&scene)?,
        Command::Scene {
            command: SceneCommand::Validate { path, json },
        } => {
            let scene = load_scene(&path)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "valid": true,
                        "scene": scene,
                    }))
                    .map_err(|e| e.to_string())?
                );
            } else {
                println!("Valid scene: {} ({} props)", scene.name, scene.props.len());
            }
        }
        Command::Asset {
            command: AssetCommand::Validate { path, json },
        } => {
            let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let report = validate_gltf(&bytes)?;
            print_asset_report(report, json)?;
        }
        Command::Model {
            command: ModelCommand::Create { spec, output, json },
        } => {
            let contents =
                fs::read_to_string(&spec).map_err(|e| format!("{}: {e}", spec.display()))?;
            let model: ModelSpec =
                serde_json::from_str(&contents).map_err(|e| format!("invalid model spec: {e}"))?;
            let glb = create_box_glb(&model)?;
            if let Some(parent) = output
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            fs::write(&output, &glb).map_err(|e| format!("{}: {e}", output.display()))?;
            let report = validate_gltf(&glb)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "output": output,
                        "model": report,
                    }))
                    .map_err(|e| e.to_string())?
                );
            } else {
                println!(
                    "Created {}: {} mesh, {} bytes",
                    output.display(),
                    report.mesh_count,
                    report.bytes
                );
            }
        }
    }
    Ok(())
}

fn print_asset_report(report: AssetReport, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "Valid {} {} asset: {} mesh(es), {} bytes",
            report.format, report.version, report.mesh_count, report.bytes
        );
    }
    Ok(())
}

fn load_scene(path: &Path) -> Result<SceneSpec, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let scene =
        parse_scene(&content).map_err(|e| format!("invalid scene {}: {e}", path.display()))?;
    validate_scene_assets(&scene, Path::new("assets"))?;
    Ok(scene)
}
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Npc;

#[derive(Component)]
struct DialogueHud;

#[derive(Resource)]
struct ActiveScene(SceneSpec);

#[derive(Resource)]
struct RuntimeStory {
    progress: GameProgress,
    frame_remainder: f32,
}

impl Default for RuntimeStory {
    fn default() -> Self {
        Self {
            progress: GameProgress::default(),
            frame_remainder: 0.0,
        }
    }
}

fn play(path: &Path) -> Result<(), String> {
    let scene = load_scene(path)?;
    let title = format!("TRS Engine — {}", scene.name);
    let asset_root = Path::new("assets")
        .canonicalize()
        .map_err(|error| format!("assets: {error}"))?
        .to_string_lossy()
        .into_owned();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title,
                        resolution: WindowResolution::new(1280, 720),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root,
                    ..default()
                }),
        )
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .insert_resource(ActiveScene(scene))
        .init_resource::<RuntimeStory>()
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                player_controller,
                npc_interaction,
                advance_story,
                animate_npc,
                cinematic_camera,
                mission_progress,
                update_hud,
            )
                .chain(),
        )
        .run();
    Ok(())
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    scene: Res<ActiveScene>,
) {
    let spec = &scene.0;
    let ground_x = spec.ground_size[0];
    let ground_z = spec.ground_size[1];
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(ground_x, ground_z))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.42, 0.22))),
        Transform::default(),
        RigidBody::Fixed,
        Collider::cuboid(ground_x * 0.5, 0.1, ground_z * 0.5),
    ));
    spawn_actor(
        &mut commands,
        &mut meshes,
        &mut materials,
        &spec.player,
        true,
    );
    spawn_actor(&mut commands, &mut meshes, &mut materials, &spec.npc, false);
    for prop in &spec.props {
        let color = prop.color;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(prop.size[0], prop.size[1], prop.size[2]))),
            MeshMaterial3d(materials.add(Color::srgb(color[0], color[1], color[2]))),
            Transform::from_xyz(prop.position[0], prop.position[1], prop.position[2]),
            RigidBody::Fixed,
            Collider::cuboid(prop.size[0] * 0.5, prop.size[1] * 0.5, prop.size[2] * 0.5),
        ));
    }
    let field = &spec.field;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(field.size[0], 0.08, field.size[1]))),
        MeshMaterial3d(materials.add(Color::srgb(0.12, 0.52, 0.2))),
        Transform::from_xyz(field.position[0], 0.04, field.position[2]),
        RigidBody::Fixed,
        Collider::cuboid(field.size[0] * 0.5, 0.04, field.size[1] * 0.5),
    ));
    if let Some(model) = &field.model {
        commands.spawn((
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(model.clone()))),
            Transform::from_xyz(field.position[0], 0.5, field.position[2]),
        ));
    } else {
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.25).mesh().uv(16, 12))),
            MeshMaterial3d(materials.add(Color::srgb(0.96, 0.9, 0.2))),
            Transform::from_xyz(field.position[0], 0.5, field.position[2]),
        ));
    }
    let camera = Transform::from_xyz(0.0, 7.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y);
    commands.spawn((Camera3d::default(), camera));
    commands.spawn((
        PointLight {
            intensity: 2_500_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 8.0, 4.0),
    ));
    commands.spawn((
        Text::new("WASD: move   E: talk to Carlos"),
        TextFont::from_font_size(24.0),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            bottom: px(24),
            ..default()
        },
        DialogueHud,
    ));
}

fn spawn_actor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    actor: &ActorSpec,
    player: bool,
) {
    let color = actor.color;
    let mut entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(actor.size[0], actor.size[1], actor.size[2]))),
        MeshMaterial3d(materials.add(Color::srgb(color[0], color[1], color[2]))),
        Transform::from_xyz(actor.position[0], actor.position[1], actor.position[2]),
    ));
    if player {
        entity.insert((
            Player,
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(actor.size[1] * 0.44, actor.size[0] * 0.4),
        ));
    } else {
        entity.insert((
            Npc,
            RigidBody::Fixed,
            Collider::capsule_y(actor.size[1] * 0.44, actor.size[0] * 0.4),
        ));
    }
}

fn player_controller(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    scene: Res<ActiveScene>,
    story: Res<RuntimeStory>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    if !matches!(story.progress.phase, GamePhase::Roam | GamePhase::FindField) {
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
        let speed = 3.0 * time.delta_secs();
        transform.translation += direction.normalize_or_zero() * speed;
        transform.translation.x = transform
            .translation
            .x
            .clamp(-scene.0.ground_size[0] * 0.5, scene.0.ground_size[0] * 0.5);
        transform.translation.z = transform
            .translation
            .z
            .clamp(-scene.0.ground_size[1] * 0.5, scene.0.ground_size[1] * 0.5);
    }
}

fn npc_interaction(
    keys: Res<ButtonInput<KeyCode>>,
    scene: Res<ActiveScene>,
    mut story: ResMut<RuntimeStory>,
    player: Query<&Transform, With<Player>>,
    npc: Query<&Transform, With<Npc>>,
) {
    if !keys.just_pressed(KeyCode::KeyE) || story.progress.phase != GamePhase::Roam {
        return;
    }
    let (Ok(player), Ok(npc)) = (player.single(), npc.single()) else {
        return;
    };
    let distance = player.translation.xz().distance(npc.translation.xz());
    if story.progress.interact(distance) {
        info!("Interaction started with {}", scene.0.npc.name);
    }
}

fn advance_story(time: Res<Time>, scene: Res<ActiveScene>, mut story: ResMut<RuntimeStory>) {
    if story.progress.phase != GamePhase::Cutscene {
        return;
    }
    story.frame_remainder += time.delta_secs() * scene.0.cutscene.fps as f32;
    let frames = story.frame_remainder.floor() as u32;
    if frames > 0 {
        story.frame_remainder -= frames as f32;
        story
            .progress
            .advance_cutscene(frames, &scene.0.cutscene, &scene.0.field.objective);
    }
}

fn animate_npc(
    time: Res<Time>,
    story: Res<RuntimeStory>,
    mut npc: Query<&mut Transform, With<Npc>>,
) {
    if let Ok(mut transform) = npc.single_mut() {
        transform.rotation = if story.progress.phase == GamePhase::Cutscene {
            Quat::from_rotation_y((time.elapsed_secs() * 4.0).sin() * 0.25)
        } else {
            Quat::IDENTITY
        };
    }
}

fn cinematic_camera(
    scene: Res<ActiveScene>,
    story: Res<RuntimeStory>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    if story.progress.phase != GamePhase::Cutscene {
        *transform = Transform::from_xyz(0.0, 7.0, 11.0).looking_at(Vec3::ZERO, Vec3::Y);
        return;
    }
    let shots = &scene.0.cutscene.shots;
    let next = shots
        .iter()
        .position(|shot| shot.frame > story.progress.frame)
        .unwrap_or(shots.len());
    let start = &shots[next.saturating_sub(1)];
    let target = shots.get(next).unwrap_or(start);
    let span = target.frame.saturating_sub(start.frame).max(1) as f32;
    let amount = ((story.progress.frame.saturating_sub(start.frame)) as f32 / span).clamp(0.0, 1.0);
    let position = Vec3::from_array(start.position).lerp(Vec3::from_array(target.position), amount);
    let focus = Vec3::from_array(start.target).lerp(Vec3::from_array(target.target), amount);
    *transform = Transform::from_translation(position).looking_at(focus, Vec3::Y);
}

fn mission_progress(
    scene: Res<ActiveScene>,
    mut story: ResMut<RuntimeStory>,
    player: Query<&Transform, With<Player>>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    story
        .progress
        .observe_player(player.translation.to_array(), &scene.0.field);
}

fn update_hud(
    scene: Res<ActiveScene>,
    story: Res<RuntimeStory>,
    mut hud: Query<&mut Text, With<DialogueHud>>,
) {
    let Ok(mut hud) = hud.single_mut() else {
        return;
    };
    hud.0 = match story.progress.phase {
        GamePhase::Roam => format!(
            "WASD: mover   E: conversar com {} (aproxime-se)",
            scene.0.npc.name
        ),
        GamePhase::Cutscene => story
            .progress
            .dialogue
            .clone()
            .unwrap_or_else(|| "Cutscene".into()),
        GamePhase::FindField => format!(
            "Objetivo: {}",
            story
                .progress
                .objective
                .as_deref()
                .unwrap_or(&scene.0.field.objective)
        ),
        GamePhase::Complete => story
            .progress
            .objective
            .clone()
            .unwrap_or_else(|| "Missão concluída!".into()),
    };
}
