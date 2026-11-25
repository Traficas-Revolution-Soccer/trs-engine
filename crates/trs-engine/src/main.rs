use bevy::{prelude::*, window::WindowResolution};
use bevy_rapier3d::prelude::*;
use clap::{Parser, Subcommand};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
use trs_engine::{
    ActorSpec, AssetReport, GamePhase, GameProgress, ModelSpec, SceneSpec, create_game_save,
    create_model_glb, parse_game_save, parse_scene, run_demo, validate_gltf, validate_scene_assets,
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
        #[arg(long, help = "Load and save mission progress at this JSON path")]
        save: Option<PathBuf>,
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
    /// Generate an original box or UV-sphere mesh as a GLB file.
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
        Command::Play { scene, save } => play(&scene, save.as_deref())?,
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
            let glb = create_model_glb(&model)?;
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
#[derive(Component, Default)]
struct PlayerMotion {
    vertical_velocity: f32,
}

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
    save_path: Option<PathBuf>,
}

impl Default for RuntimeStory {
    fn default() -> Self {
        Self {
            progress: GameProgress::default(),
            frame_remainder: 0.0,
            save_path: None,
        }
    }
}

#[derive(Resource)]
struct InitialPlayerPosition([f32; 3]);

fn play(path: &Path, save_path: Option<&Path>) -> Result<(), String> {
    let scene = load_scene(path)?;
    let mut story = RuntimeStory {
        save_path: save_path.map(Path::to_path_buf),
        ..default()
    };
    let player_position = if let Some(save_path) = save_path.filter(|path| path.exists()) {
        let input = fs::read_to_string(save_path)
            .map_err(|error| format!("{}: {error}", save_path.display()))?;
        let save = parse_game_save(&input, &scene)?;
        story.progress = save.progress;
        save.player_position
    } else {
        scene.player.position
    };
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
        .insert_resource(InitialPlayerPosition(player_position))
        .insert_resource(story)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                player_controller,
                save_load_controls,
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
    initial_player_position: Res<InitialPlayerPosition>,
) {
    let spec = &scene.0;
    let ground_x = spec.ground_size[0];
    let ground_z = spec.ground_size[1];
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(ground_x, ground_z))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.42, 0.22))),
        Transform::default(),
        RigidBody::Fixed,
        Collider::cuboid(ground_x * 0.5, 0.01, ground_z * 0.5),
    ));
    for stripe in 0..8 {
        spawn_static_box(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(0.0, 0.015, -10.0 + stripe as f32 * 2.8),
            Vec3::new(ground_x, 0.02, 1.35),
            if stripe % 2 == 0 {
                Color::srgb(0.19, 0.44, 0.22)
            } else {
                Color::srgb(0.17, 0.40, 0.20)
            },
        );
    }
    for (position, scale) in [
        (Vec3::new(-13.0, 0.0, -3.0), 1.0),
        (Vec3::new(13.0, 0.0, -3.0), 1.0),
        (Vec3::new(-13.0, 0.0, 6.0), 0.85),
        (Vec3::new(14.0, 0.0, 9.0), 0.9),
    ] {
        spawn_tree(&mut commands, &mut meshes, &mut materials, position, scale);
    }
    spawn_actor(
        &mut commands,
        &mut meshes,
        &mut materials,
        &spec.player,
        true,
        Some(initial_player_position.0),
    );
    spawn_actor(
        &mut commands,
        &mut meshes,
        &mut materials,
        &spec.npc,
        false,
        None,
    );
    for prop in &spec.props {
        let color = prop.color;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(prop.size[0], prop.size[1], prop.size[2]))),
            MeshMaterial3d(materials.add(Color::srgb(color[0], color[1], color[2]))),
            Transform::from_xyz(prop.position[0], prop.position[1], prop.position[2]),
            RigidBody::Fixed,
            Collider::cuboid(prop.size[0] * 0.5, prop.size[1] * 0.5, prop.size[2] * 0.5),
        ));
        if prop.size[0] > 2.0 && prop.size[1] > 2.0 && prop.size[2] > 2.0 {
            spawn_house_details(
                &mut commands,
                &mut meshes,
                &mut materials,
                prop.position,
                prop.size,
            );
        }
    }
    let field = &spec.field;
    spawn_pitch_markings(
        &mut commands,
        &mut meshes,
        &mut materials,
        field.position,
        field.size,
    );
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(field.size[0], 0.08, field.size[1]))),
        MeshMaterial3d(materials.add(Color::srgb(0.12, 0.52, 0.2))),
        Transform::from_xyz(field.position[0], 0.04, field.position[2]),
        RigidBody::Fixed,
        Collider::cuboid(field.size[0] * 0.5, 0.04, field.size[1] * 0.5),
        Sensor,
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
    let player_position = Vec3::from_array(initial_player_position.0);
    let camera_position = player_position + Vec3::new(0.0, 5.2, 8.0);
    let camera = Transform::from_translation(camera_position)
        .looking_at(player_position + Vec3::Y * 0.7, Vec3::Y);
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
        Text::new("WASD move · Space jump · E interact · F5 save · F9 load"),
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
fn spawn_static_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    size: Vec3,
    color: Color,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(materials.add(color)),
        Transform::from_translation(position),
    ));
}

fn spawn_static_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    scale: Vec3,
    color: Color,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(16, 12))),
        MeshMaterial3d(materials.add(color)),
        Transform::from_translation(position).with_scale(scale),
    ));
}

fn spawn_tree(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    scale: f32,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.18 * scale, 1.55 * scale))),
        MeshMaterial3d(materials.add(Color::srgb(0.31, 0.17, 0.08))),
        Transform::from_xyz(position.x, 0.78 * scale, position.z),
    ));
    for (offset, size, color) in [
        (
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::splat(1.55),
            Color::srgb(0.12, 0.36, 0.16),
        ),
        (
            Vec3::new(-0.48, 1.8, 0.08),
            Vec3::splat(1.05),
            Color::srgb(0.16, 0.43, 0.18),
        ),
        (
            Vec3::new(0.46, 1.82, -0.1),
            Vec3::splat(1.08),
            Color::srgb(0.19, 0.47, 0.2),
        ),
    ] {
        spawn_static_sphere(
            commands,
            meshes,
            materials,
            position + offset * scale,
            size * scale,
            color,
        );
    }
}

fn spawn_house_details(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: [f32; 3],
    size: [f32; 3],
) {
    let center = Vec3::from_array(position);
    let (width, height, depth) = (size[0], size[1], size[2]);
    let front = center.z + depth * 0.5;
    spawn_static_box(
        commands,
        meshes,
        materials,
        center + Vec3::Y * (height * 0.5 + 0.17),
        Vec3::new(width + 0.65, 0.34, depth + 0.65),
        Color::srgb(0.48, 0.14, 0.09),
    );
    spawn_static_box(
        commands,
        meshes,
        materials,
        Vec3::new(center.x, 0.12, front + 0.12),
        Vec3::new(width + 0.18, 0.2, 0.3),
        Color::srgb(0.84, 0.76, 0.61),
    );
    spawn_static_box(
        commands,
        meshes,
        materials,
        Vec3::new(center.x, height * 0.52, front + 0.075),
        Vec3::new(0.84, height * 0.55, 0.12),
        Color::srgb(0.22, 0.11, 0.07),
    );
    spawn_static_box(
        commands,
        meshes,
        materials,
        Vec3::new(center.x, height * 0.52, front + 0.15),
        Vec3::new(0.66, height * 0.49, 0.08),
        Color::srgb(0.45, 0.24, 0.13),
    );
    for side in [-1.0, 1.0] {
        let window_x = center.x + side * width * 0.29;
        spawn_static_box(
            commands,
            meshes,
            materials,
            Vec3::new(window_x, height * 0.68, front + 0.08),
            Vec3::new(0.92, 0.95, 0.12),
            Color::srgb(0.91, 0.78, 0.54),
        );
        spawn_static_box(
            commands,
            meshes,
            materials,
            Vec3::new(window_x, height * 0.68, front + 0.16),
            Vec3::new(0.72, 0.73, 0.06),
            Color::srgb(0.18, 0.52, 0.63),
        );
        spawn_static_box(
            commands,
            meshes,
            materials,
            Vec3::new(window_x, height * 0.68, front + 0.21),
            Vec3::new(0.07, 0.73, 0.03),
            Color::srgb(0.91, 0.78, 0.54),
        );
        spawn_static_box(
            commands,
            meshes,
            materials,
            Vec3::new(window_x, height * 0.68, front + 0.21),
            Vec3::new(0.72, 0.07, 0.03),
            Color::srgb(0.91, 0.78, 0.54),
        );
    }
}

fn spawn_pitch_markings(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: [f32; 3],
    size: [f32; 2],
) {
    let (x, y, z) = (position[0], 0.091, position[2]);
    let white = Color::srgb(0.92, 0.91, 0.81);
    let (half_x, half_z) = (size[0] * 0.5 - 0.12, size[1] * 0.5 - 0.12);
    let line_width = 0.055;
    for (offset, dimensions) in [
        (
            Vec3::new(0.0, 0.0, -half_z),
            Vec3::new(size[0], 0.025, line_width),
        ),
        (
            Vec3::new(0.0, 0.0, half_z),
            Vec3::new(size[0], 0.025, line_width),
        ),
        (
            Vec3::new(-half_x, 0.0, 0.0),
            Vec3::new(line_width, 0.025, size[1]),
        ),
        (
            Vec3::new(half_x, 0.0, 0.0),
            Vec3::new(line_width, 0.025, size[1]),
        ),
        (Vec3::ZERO, Vec3::new(line_width, 0.025, size[1])),
    ] {
        spawn_static_box(
            commands,
            meshes,
            materials,
            Vec3::new(x, y, z) + offset,
            dimensions,
            white,
        );
    }
    let radius = size[1].min(size[0]) * 0.15;
    for segment in 0..24 {
        let angle = std::f32::consts::TAU * segment as f32 / 24.0;
        let mut transform =
            Transform::from_xyz(x + angle.cos() * radius, y, z + angle.sin() * radius);
        transform.rotation = Quat::from_rotation_y(-angle);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(radius * 0.28, 0.025, line_width))),
            MeshMaterial3d(materials.add(white)),
            transform,
        ));
    }
    spawn_goal(
        commands,
        meshes,
        materials,
        Vec3::new(x - half_x, 0.0, z),
        half_z,
    );
    spawn_goal(
        commands,
        meshes,
        materials,
        Vec3::new(x + half_x, 0.0, z),
        half_z,
    );
}

fn spawn_goal(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    field_half_z: f32,
) {
    let goal_width = field_half_z * 0.6;
    let frame = Color::srgb(0.95, 0.94, 0.87);
    for side in [-1.0, 1.0] {
        spawn_static_box(
            commands,
            meshes,
            materials,
            center + Vec3::new(0.0, 0.85, side * goal_width * 0.5),
            Vec3::new(0.1, 1.7, 0.1),
            frame,
        );
    }
    spawn_static_box(
        commands,
        meshes,
        materials,
        center + Vec3::new(0.0, 1.68, 0.0),
        Vec3::new(0.1, 0.1, goal_width),
        frame,
    );
}

fn spawn_actor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    actor: &ActorSpec,
    player: bool,
    position: Option<[f32; 3]>,
) {
    let radius = actor.size[0].min(actor.size[2]) * 0.4;
    let half_segment = (actor.size[1] * 0.5 - radius).max(0.0);
    let root = commands
        .spawn((
            Transform::from_translation(Vec3::from_array(position.unwrap_or(actor.position))),
            Visibility::Visible,
        ))
        .id();
    let color = Color::srgb(actor.color[0], actor.color[1], actor.color[2]);
    let width = actor.size[0];
    let height = actor.size[1];
    let depth = actor.size[2];
    let skin = Color::srgb(0.82, 0.56, 0.39);
    let shorts = if player {
        Color::srgb(0.07, 0.16, 0.34)
    } else {
        Color::srgb(0.23, 0.18, 0.15)
    };
    let hair = if player {
        Color::srgb(0.09, 0.055, 0.035)
    } else {
        Color::srgb(0.18, 0.085, 0.035)
    };
    spawn_actor_box(
        commands,
        meshes,
        materials,
        root,
        Vec3::new(0.0, height * 0.06, 0.0),
        Vec3::new(width * 0.72, height * 0.42, depth * 0.62),
        color,
    );
    spawn_actor_sphere(
        commands,
        meshes,
        materials,
        root,
        Vec3::new(0.0, -height * 0.2, 0.0),
        Vec3::new(width * 0.35, height * 0.12, depth * 0.3),
        shorts,
    );
    spawn_actor_sphere(
        commands,
        meshes,
        materials,
        root,
        Vec3::new(0.0, height * 0.3, 0.0),
        Vec3::new(width * 0.12, height * 0.075, depth * 0.12),
        skin,
    );
    spawn_actor_sphere(
        commands,
        meshes,
        materials,
        root,
        Vec3::new(0.0, height * 0.39, 0.0),
        Vec3::new(width * 0.27, height * 0.14, depth * 0.23),
        skin,
    );
    spawn_actor_sphere(
        commands,
        meshes,
        materials,
        root,
        Vec3::new(0.0, height * 0.48, -depth * 0.035),
        Vec3::new(width * 0.25, height * 0.08, depth * 0.22),
        hair,
    );
    for side in [-1.0, 1.0] {
        spawn_actor_sphere(
            commands,
            meshes,
            materials,
            root,
            Vec3::new(side * width * 0.4, height * 0.03, 0.0),
            Vec3::new(width * 0.12, height * 0.15, depth * 0.12),
            color,
        );
        spawn_actor_sphere(
            commands,
            meshes,
            materials,
            root,
            Vec3::new(side * width * 0.4, -height * 0.14, 0.0),
            Vec3::new(width * 0.1, height * 0.12, depth * 0.11),
            skin,
        );
        spawn_actor_sphere(
            commands,
            meshes,
            materials,
            root,
            Vec3::new(side * width * 0.17, -height * 0.24, 0.0),
            Vec3::new(width * 0.14, height * 0.14, depth * 0.14),
            shorts,
        );
        spawn_actor_sphere(
            commands,
            meshes,
            materials,
            root,
            Vec3::new(side * width * 0.17, -height * 0.39, 0.0),
            Vec3::new(width * 0.115, height * 0.13, depth * 0.12),
            Color::srgb(0.13, 0.22, 0.34),
        );
        spawn_actor_box(
            commands,
            meshes,
            materials,
            root,
            Vec3::new(side * width * 0.17, -height * 0.47, depth * 0.1),
            Vec3::new(width * 0.3, height * 0.09, depth * 0.42),
            Color::srgb(0.08, 0.075, 0.07),
        );
        spawn_actor_sphere(
            commands,
            meshes,
            materials,
            root,
            Vec3::new(side * width * 0.105, height * 0.4, depth * 0.2),
            Vec3::new(width * 0.04, height * 0.035, depth * 0.025),
            Color::WHITE,
        );
        spawn_actor_sphere(
            commands,
            meshes,
            materials,
            root,
            Vec3::new(side * width * 0.105, height * 0.4, depth * 0.225),
            Vec3::new(width * 0.02, height * 0.025, depth * 0.018),
            Color::srgb(0.08, 0.06, 0.04),
        );
    }
    let mut entity = commands.entity(root);
    if player {
        entity.insert((
            Player,
            PlayerMotion::default(),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(half_segment, radius),
            KinematicCharacterController {
                snap_to_ground: Some(CharacterLength::Absolute(0.05)),
                ..default()
            },
        ));
    } else {
        entity.insert((
            Npc,
            RigidBody::Fixed,
            Collider::capsule_y(half_segment, radius),
        ));
    }
}

fn spawn_actor_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
    position: Vec3,
    size: Vec3,
    color: Color,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(materials.add(color)),
        Transform::from_translation(position),
        ChildOf(parent),
    ));
}

fn spawn_actor_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
    position: Vec3,
    scale: Vec3,
    color: Color,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(16, 12))),
        MeshMaterial3d(materials.add(color)),
        Transform::from_translation(position).with_scale(scale),
        ChildOf(parent),
    ));
}

fn player_controller(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    scene: Res<ActiveScene>,
    story: Res<RuntimeStory>,
    mut player: Query<
        (
            &mut Transform,
            &mut KinematicCharacterController,
            &mut PlayerMotion,
            Option<&KinematicCharacterControllerOutput>,
        ),
        With<Player>,
    >,
) {
    let Ok((mut transform, mut controller, mut motion, output)) = player.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let can_control = matches!(story.progress.phase, GamePhase::Roam | GamePhase::FindField);
    let mut direction = Vec3::ZERO;
    if can_control {
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
    }
    if direction.length_squared() > 0.0 {
        transform.rotation = Quat::from_rotation_y(direction.x.atan2(-direction.z));
    }

    let grounded = output.is_some_and(|output| output.grounded);
    if grounded && motion.vertical_velocity < 0.0 {
        motion.vertical_velocity = 0.0;
    }
    if can_control && grounded && keys.just_pressed(KeyCode::Space) {
        motion.vertical_velocity = 4.5;
    }
    motion.vertical_velocity -= 9.81 * dt;

    let radius = scene.0.player.size[0].min(scene.0.player.size[2]) * 0.4;
    let destination = transform.translation + direction.normalize_or_zero() * 3.0 * dt;
    let destination = Vec3::new(
        destination.x.clamp(
            -scene.0.ground_size[0] * 0.5 + radius,
            scene.0.ground_size[0] * 0.5 - radius,
        ),
        transform.translation.y,
        destination.z.clamp(
            -scene.0.ground_size[1] * 0.5 + radius,
            scene.0.ground_size[1] * 0.5 - radius,
        ),
    );
    controller.translation =
        Some(destination - transform.translation + Vec3::Y * motion.vertical_velocity * dt);
}

fn save_load_controls(
    keys: Res<ButtonInput<KeyCode>>,
    scene: Res<ActiveScene>,
    mut story: ResMut<RuntimeStory>,
    mut player: Query<(&mut Transform, &mut PlayerMotion), With<Player>>,
) {
    if keys.just_pressed(KeyCode::F5) {
        let Some(path) = story.save_path.clone() else {
            warn!("Pass --save <path> to enable mission saving");
            return;
        };
        let Ok((transform, _)) = player.single_mut() else {
            return;
        };
        let position = transform.translation.to_array();
        match create_game_save(&scene.0, &story.progress, position)
            .and_then(|contents| write_save_atomically(&path, &contents))
        {
            Ok(()) => info!("Mission saved to {}", path.display()),
            Err(error) => error!("Could not save mission: {error}"),
        }
    }
    if keys.just_pressed(KeyCode::F9) {
        let Some(path) = story.save_path.clone() else {
            warn!("Pass --save <path> to enable mission loading");
            return;
        };
        let input = match fs::read_to_string(&path) {
            Ok(input) => input,
            Err(error) => {
                error!("Could not read save {}: {error}", path.display());
                return;
            }
        };
        let save = match parse_game_save(&input, &scene.0) {
            Ok(save) => save,
            Err(error) => {
                error!("Could not load mission save: {error}");
                return;
            }
        };
        let Ok((mut transform, mut motion)) = player.single_mut() else {
            return;
        };
        transform.translation = Vec3::from_array(save.player_position);
        motion.vertical_velocity = 0.0;
        story.progress = save.progress;
        story.frame_remainder = 0.0;
        info!("Mission loaded from {}", path.display());
    }
}

fn write_save_atomically(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut temporary_name = path.as_os_str().to_owned();
    temporary_name.push(".tmp");
    let temporary_path = PathBuf::from(temporary_name);
    fs::write(&temporary_path, contents)
        .map_err(|error| format!("{}: {error}", temporary_path.display()))?;
    fs::rename(&temporary_path, path).map_err(|error| {
        format!(
            "could not replace {} with {}: {error}",
            path.display(),
            temporary_path.display()
        )
    })
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
    time: Res<Time>,
    scene: Res<ActiveScene>,
    story: Res<RuntimeStory>,
    player: Query<&Transform, (With<Player>, Without<Camera3d>)>,
    mut camera: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    if story.progress.phase != GamePhase::Cutscene {
        let Ok(player) = player.single() else {
            return;
        };
        let focus = player.translation + Vec3::Y * 0.45;
        let position = player.translation + Vec3::new(0.0, 5.2, 8.0);
        let target = Transform::from_translation(position).looking_at(focus, Vec3::Y);
        let smoothing = 1.0 - (-time.delta_secs() * 5.0).exp();
        transform.translation = transform.translation.lerp(target.translation, smoothing);
        transform.rotation = transform.rotation.slerp(target.rotation, smoothing);
        return;
    }

    let shots = &scene.0.cutscene.shots;
    let frame = story.progress.frame as f32 + story.frame_remainder;
    let next = shots
        .iter()
        .position(|shot| shot.frame as f32 > frame)
        .unwrap_or(shots.len());
    let start = &shots[next.saturating_sub(1)];
    let target = shots.get(next).unwrap_or(start);
    let span = target.frame.saturating_sub(start.frame).max(1) as f32;
    let amount = ((frame - start.frame as f32) / span).clamp(0.0, 1.0);
    let eased = amount * amount * (3.0 - 2.0 * amount);
    let mut position =
        Vec3::from_array(start.position).lerp(Vec3::from_array(target.position), eased);
    position.y += (amount * std::f32::consts::PI).sin() * 0.3;
    let focus = Vec3::from_array(start.target).lerp(Vec3::from_array(target.target), eased);
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

#[cfg(test)]
mod tests {
    use super::*;

    use bevy::time::{TimePlugin, TimeUpdateStrategy};
    #[test]
    fn cinematic_camera_eases_through_authored_shots() {
        let scene =
            parse_scene(include_str!("../../../assets/scenes/bairro-da-bola.json")).unwrap();
        let mut story = RuntimeStory::default();
        assert!(story.progress.interact(1.0));
        story
            .progress
            .advance_cutscene(50, &scene.cutscene, &scene.field.objective);
        story.frame_remainder = 0.5;

        let mut app = App::new();
        app.insert_resource(ActiveScene(scene))
            .insert_resource(story)
            .insert_resource(Time::<()>::default())
            .add_systems(Update, cinematic_camera);
        app.world_mut()
            .spawn((Player, Transform::from_xyz(-2.0, 0.9, 0.0)));
        let camera = app
            .world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(0.0, 7.0, 11.0)))
            .id();

        app.update();

        let transform = app.world().get::<Transform>(camera).unwrap();
        assert!((2.4..4.2).contains(&transform.translation.x));
        assert!(transform.translation.y > 2.7);
    }

    #[test]
    fn save_hotkeys_round_trip_mission_state_and_player_position() {
        let scene =
            parse_scene(include_str!("../../../assets/scenes/bairro-da-bola.json")).unwrap();
        let save_path =
            std::env::temp_dir().join(format!("trs-engine-save-{}.json", std::process::id()));
        let mut story = RuntimeStory {
            save_path: Some(save_path.clone()),
            ..default()
        };
        assert!(story.progress.interact(1.0));
        story
            .progress
            .advance_cutscene(50, &scene.cutscene, &scene.field.objective);
        let mut app = App::new();
        app.insert_resource(ActiveScene(scene))
            .insert_resource(story)
            .insert_resource(ButtonInput::<KeyCode>::default())
            .add_systems(Update, save_load_controls);
        let player = app
            .world_mut()
            .spawn((
                Player,
                PlayerMotion::default(),
                Transform::from_xyz(-1.0, 0.9, 2.0),
            ))
            .id();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F5);
        app.update();
        *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = ButtonInput::default();
        assert!(save_path.exists());

        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = Vec3::new(4.0, 0.9, 3.0);
        app.world_mut().resource_mut::<RuntimeStory>().progress = GameProgress::default();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F9);
        app.update();

        let transform = app.world().get::<Transform>(player).unwrap();
        assert_eq!(transform.translation, Vec3::new(-1.0, 0.9, 2.0));
        let progress = &app.world().resource::<RuntimeStory>().progress;
        assert_eq!(progress.phase, GamePhase::Cutscene);
        assert_eq!(progress.frame, 50);
        let _ = fs::remove_file(save_path);
    }

    #[test]
    fn character_controller_passes_field_sensor_and_stops_at_wall() {
        let mut app = App::new();
        app.add_plugins((
            TransformPlugin,
            TimePlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 60.0),
        ));
        app.finish();
        app.world_mut().spawn((
            RigidBody::Fixed,
            Sensor,
            Collider::cuboid(0.5, 1.0, 1.0),
            Transform::from_xyz(1.0, 1.0, 0.0),
        ));
        app.world_mut().spawn((
            RigidBody::Fixed,
            Collider::cuboid(0.1, 2.0, 2.0),
            Transform::from_xyz(3.0, 1.0, 0.0),
        ));
        let character = app
            .world_mut()
            .spawn((
                Collider::capsule_y(0.6, 0.3),
                KinematicCharacterController::default(),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ))
            .id();

        app.update();
        app.update();
        app.world_mut()
            .get_mut::<KinematicCharacterController>(character)
            .unwrap()
            .translation = Some(Vec3::X * 5.0);
        app.update();

        let position = app
            .world()
            .get::<Transform>(character)
            .unwrap()
            .translation
            .x;
        let output = app
            .world()
            .get::<KinematicCharacterControllerOutput>(character)
            .unwrap();
        assert!(
            position > 2.0,
            "sensor field blocked movement at {position}"
        );
        assert!(position < 2.9, "character crossed wall at {position}");
        assert!(!output.collisions.is_empty());
    }

    #[test]
    fn player_controller_jumps_and_lands_on_ground() {
        let scene =
            parse_scene(include_str!("../../../assets/scenes/bairro-da-bola.json")).unwrap();
        let mut app = App::new();
        app.add_plugins((
            TransformPlugin,
            TimePlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 60.0),
        ))
        .insert_resource(ActiveScene(scene))
        .init_resource::<RuntimeStory>()
        .insert_resource(ButtonInput::<KeyCode>::default())
        .add_systems(Update, player_controller);
        app.finish();
        app.world_mut().spawn((
            RigidBody::Fixed,
            Collider::cuboid(16.0, 0.25, 12.0),
            Transform::from_xyz(0.0, -0.25, 0.0),
        ));
        let player = app
            .world_mut()
            .spawn((
                Player,
                PlayerMotion::default(),
                RigidBody::KinematicPositionBased,
                Collider::capsule_y(0.62, 0.28),
                KinematicCharacterController {
                    snap_to_ground: Some(CharacterLength::Absolute(0.05)),
                    ..default()
                },
                Transform::from_xyz(0.0, 1.5, 0.0),
            ))
            .id();

        for _ in 0..45 {
            app.update();
        }
        let before = app.world().get::<Transform>(player).unwrap().translation.y;
        assert!(
            app.world()
                .get::<KinematicCharacterControllerOutput>(player)
                .unwrap()
                .grounded,
            "player did not reach the ground before jumping at y={before}"
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = ButtonInput::default();
        let airborne = app.world().get::<Transform>(player).unwrap().translation.y;
        assert!(
            airborne > before + 0.05,
            "jump did not lift player: {airborne}"
        );

        for _ in 0..90 {
            app.update();
        }
        let landed = app.world().get::<Transform>(player).unwrap().translation.y;
        let output = app
            .world()
            .get::<KinematicCharacterControllerOutput>(player)
            .unwrap();
        assert!(output.grounded, "player did not land at y={landed}");
        assert!(
            (landed - 0.9).abs() < 0.1,
            "ground contact drifted to y={landed}"
        );
    }
}
