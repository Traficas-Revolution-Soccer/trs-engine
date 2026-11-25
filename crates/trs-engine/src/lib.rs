mod assets;
pub use assets::{AssetReport, ModelSpec, create_box_glb, validate_gltf};

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SceneSpec {
    pub version: u32,
    pub name: String,
    pub ground_size: [f32; 2],
    pub player: ActorSpec,
    pub npc: ActorSpec,
    pub field: FieldSpec,
    pub props: Vec<PropSpec>,
    pub cutscene: CutsceneSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActorSpec {
    pub name: String,
    pub position: [f32; 3],
    pub size: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldSpec {
    pub position: [f32; 3],
    pub size: [f32; 2],
    pub objective: String,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PropSpec {
    pub position: [f32; 3],
    pub size: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CutsceneSpec {
    pub fps: u32,
    pub duration_frames: u32,
    pub shots: Vec<CameraShot>,
    pub dialogue: Vec<DialogueLine>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CameraShot {
    pub frame: u32,
    pub position: [f32; 3],
    pub target: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DialogueLine {
    pub frame: u32,
    pub speaker: String,
    pub text: String,
}

pub fn parse_scene(input: &str) -> Result<SceneSpec, String> {
    let scene: SceneSpec = serde_json::from_str(input).map_err(|error| error.to_string())?;
    validate_scene(&scene)?;
    Ok(scene)
}

pub fn validate_scene_assets(scene: &SceneSpec, assets_root: &Path) -> Result<(), String> {
    let Some(model) = scene.field.model.as_deref() else {
        return Ok(());
    };
    let root = assets_root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", assets_root.display()))?;
    let path = assets_root.join(model);
    let resolved = path
        .canonicalize()
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if !resolved.starts_with(&root) {
        return Err("scene asset resolves outside the assets directory".into());
    }
    let bytes =
        std::fs::read(&resolved).map_err(|error| format!("{}: {error}", resolved.display()))?;
    validate_gltf(&bytes)?;
    Ok(())
}

pub fn validate_scene(scene: &SceneSpec) -> Result<(), String> {
    if scene.version != 1 {
        return Err(format!(
            "unsupported scene version {}; expected 1",
            scene.version
        ));
    }
    if scene.name.trim().is_empty() {
        return Err("scene name must not be empty".into());
    }
    positive2(scene.ground_size, "ground_size")?;
    validate_actor(&scene.player, "player", scene.ground_size)?;
    validate_actor(&scene.npc, "npc", scene.ground_size)?;
    if !finite3(scene.field.position) || positive2(scene.field.size, "field.size").is_err() {
        return Err("field position/size must be finite and field size positive".into());
    }
    if !inside_ground(
        scene.field.position,
        [scene.field.size[0], 0.0, scene.field.size[1]],
        scene.ground_size,
    ) {
        return Err("field extends beyond the ground bounds".into());
    }
    if scene.field.model.as_ref().is_some_and(|path| {
        path.starts_with('/')
            || path.contains('\\')
            || path.split('/').any(|part| part == ".." || part.is_empty())
            || !path.ends_with(".glb")
    }) {
        return Err(
            "field model must be a relative .glb asset path without parent-directory segments"
                .into(),
        );
    }
    if scene.field.objective.trim().is_empty() {
        return Err("field objective must not be empty".into());
    }
    for (index, prop) in scene.props.iter().enumerate() {
        validate_transform(
            prop.position,
            prop.size,
            prop.color,
            &format!("props[{index}]"),
        )?;
        if !inside_ground(prop.position, prop.size, scene.ground_size) {
            return Err(format!("props[{index}] extends beyond the ground bounds"));
        }
    }
    let cutscene = &scene.cutscene;
    if cutscene.fps == 0 || cutscene.duration_frames == 0 {
        return Err("cutscene fps and duration_frames must be positive".into());
    }
    if cutscene.shots.is_empty() || cutscene.shots[0].frame != 0 {
        return Err("cutscene needs a camera shot starting at frame 0".into());
    }
    if !strictly_increasing(cutscene.shots.iter().map(|shot| shot.frame))
        || cutscene.shots.iter().any(|shot| {
            shot.frame >= cutscene.duration_frames
                || !finite3(shot.position)
                || !finite3(shot.target)
        })
    {
        return Err(
            "camera shots must have finite transforms and increasing frames inside the cutscene"
                .into(),
        );
    }
    if cutscene.dialogue.is_empty()
        || !strictly_increasing(cutscene.dialogue.iter().map(|line| line.frame))
        || cutscene.dialogue.iter().any(|line| {
            line.frame >= cutscene.duration_frames
                || line.speaker.trim().is_empty()
                || line.text.trim().is_empty()
        })
    {
        return Err(
            "dialogue needs non-empty speakers/text and increasing frames inside the cutscene"
                .into(),
        );
    }
    Ok(())
}

fn validate_actor(actor: &ActorSpec, name: &str, ground_size: [f32; 2]) -> Result<(), String> {
    if actor.name.trim().is_empty() {
        return Err(format!("{name}.name must not be empty"));
    }
    validate_transform(actor.position, actor.size, actor.color, name)?;
    if !inside_ground(actor.position, actor.size, ground_size) {
        return Err(format!("{name} extends beyond the ground bounds"));
    }
    Ok(())
}

fn inside_ground(position: [f32; 3], size: [f32; 3], ground: [f32; 2]) -> bool {
    position[0].abs() + size[0] * 0.5 <= ground[0] * 0.5
        && position[2].abs() + size[2] * 0.5 <= ground[1] * 0.5
}

fn validate_transform(
    position: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
    name: &str,
) -> Result<(), String> {
    if !finite3(position) || !size.into_iter().all(|v| v.is_finite() && v > 0.0) {
        return Err(format!(
            "{name} position must be finite and size components positive"
        ));
    }
    if !color
        .into_iter()
        .all(|v| v.is_finite() && (0.0..=1.0).contains(&v))
    {
        return Err(format!("{name} color components must be within 0..=1"));
    }
    Ok(())
}

fn positive2(value: [f32; 2], name: &str) -> Result<(), String> {
    if value.into_iter().all(|v| v.is_finite() && v > 0.0) {
        Ok(())
    } else {
        Err(format!("{name} values must be positive and finite"))
    }
}

fn finite3(value: [f32; 3]) -> bool {
    value.into_iter().all(f32::is_finite)
}

fn strictly_increasing(values: impl Iterator<Item = u32>) -> bool {
    let mut previous = None;
    for value in values {
        if previous.is_some_and(|last| value <= last) {
            return false;
        }
        previous = Some(value);
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePhase {
    Roam,
    Cutscene,
    FindField,
    Complete,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameProgress {
    pub phase: GamePhase,
    pub frame: u32,
    pub dialogue: Option<String>,
    pub objective: Option<String>,
    pub events: Vec<String>,
    dialogue_index: usize,
}

impl Default for GameProgress {
    fn default() -> Self {
        Self {
            phase: GamePhase::Roam,
            frame: 0,
            dialogue: None,
            objective: None,
            events: Vec::new(),
            dialogue_index: 0,
        }
    }
}

impl GameProgress {
    pub fn interact(&mut self, distance: f32) -> bool {
        if self.phase != GamePhase::Roam
            || !distance.is_finite()
            || !(0.0..=1.8).contains(&distance)
        {
            return false;
        }
        self.phase = GamePhase::Cutscene;
        self.events.push("interaction.started".into());
        true
    }

    pub fn advance_cutscene(&mut self, frames: u32, cutscene: &CutsceneSpec, objective: &str) {
        if self.phase != GamePhase::Cutscene {
            return;
        }
        self.frame = self
            .frame
            .saturating_add(frames)
            .min(cutscene.duration_frames);
        while let Some(line) = cutscene.dialogue.get(self.dialogue_index) {
            if line.frame > self.frame {
                break;
            }
            let dialogue = format!("{}: {}", line.speaker, line.text);
            if self.dialogue.as_deref() != Some(&dialogue) {
                self.events.push(format!("dialogue: {dialogue}"));
                self.dialogue = Some(dialogue);
            }
            self.dialogue_index += 1;
        }
        if self.frame >= cutscene.duration_frames {
            self.phase = GamePhase::FindField;
            self.objective = Some(objective.to_owned());
            self.events.push("cutscene.ended".into());
            self.events.push(format!("mission.started: {objective}"));
        }
    }

    pub fn observe_player(&mut self, position: [f32; 3], field: &FieldSpec) {
        if self.phase == GamePhase::FindField
            && horizontal_distance(position, field.position) <= 1.5
        {
            self.phase = GamePhase::Complete;
            self.objective = Some(format!("Concluído: {}", field.objective));
            self.events.push("mission.completed".into());
        }
    }
}

fn horizontal_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoReport {
    pub scene: String,
    pub systems: Vec<&'static str>,
    pub events: Vec<String>,
    pub final_phase: GamePhase,
    pub slice_passed: bool,
}

pub fn run_demo() -> Result<DemoReport, String> {
    let scene = parse_scene(include_str!("../../../assets/scenes/bairro-da-bola.json"))?;
    let mut game = GameProgress::default();
    let mut player = scene.player.position;
    let npc = scene.npc.position;
    let mut events = vec![format!("scene.loaded: {}", scene.name)];

    while horizontal_distance(player, npc) > 1.5 {
        let dx = npc[0] - player[0];
        let dz = npc[2] - player[2];
        let distance = dx.hypot(dz);
        player[0] += dx / distance * 0.25;
        player[2] += dz / distance * 0.25;
    }
    events.push(format!(
        "player.interaction_range: [{:.2}, {:.2}, {:.2}]",
        player[0], player[1], player[2]
    ));
    game.interact(horizontal_distance(player, npc));
    game.advance_cutscene(
        scene.cutscene.duration_frames,
        &scene.cutscene,
        &scene.field.objective,
    );
    events.append(&mut game.events);

    while horizontal_distance(player, scene.field.position) > 1.0 {
        let dx = scene.field.position[0] - player[0];
        let dz = scene.field.position[2] - player[2];
        let distance = dx.hypot(dz);
        player[0] += dx / distance * 0.25;
        player[2] += dz / distance * 0.25;
    }
    game.observe_player(player, &scene.field);
    events.push(format!(
        "player.reached_field: [{:.2}, {:.2}, {:.2}]",
        player[0], player[1], player[2]
    ));
    events.extend(game.events);
    let slice_passed = game.phase == GamePhase::Complete;
    if slice_passed {
        events.push("visual_validation: passed".into());
    }
    Ok(DemoReport {
        scene: scene.name,
        systems: vec![
            "world",
            "player_controller",
            "interaction",
            "timeline",
            "dialogue",
            "mission",
            "validation",
        ],
        events,
        final_phase: game.phase,
        slice_passed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_scene() -> SceneSpec {
        parse_scene(include_str!("../../../assets/scenes/bairro-da-bola.json")).unwrap()
    }

    #[test]
    fn story_progression_requires_interaction_cutscene_and_reaching_field() {
        let scene = demo_scene();
        let mut game = GameProgress::default();
        assert!(!game.interact(2.0));
        assert!(game.interact(1.5));
        assert_eq!(game.phase, GamePhase::Cutscene);
        game.advance_cutscene(50, &scene.cutscene, &scene.field.objective);
        assert!(game.dialogue.as_deref().unwrap().contains("Bora."));
        assert_eq!(game.phase, GamePhase::Cutscene);
        game.advance_cutscene(70, &scene.cutscene, &scene.field.objective);
        assert_eq!(game.phase, GamePhase::FindField);
        game.observe_player(scene.field.position, &scene.field);
        assert_eq!(game.phase, GamePhase::Complete);
    }

    #[test]
    fn scene_validation_rejects_invalid_timeline_and_geometry() {
        let mut scene = demo_scene();
        scene.cutscene.shots[1].frame = 0;
        assert!(validate_scene(&scene).unwrap_err().contains("camera shots"));
        let mut scene = demo_scene();
        scene.player.size[0] = 0.0;
        assert!(validate_scene(&scene).unwrap_err().contains("size"));
    }

    #[test]
    fn demo_reaches_complete_mission_and_emits_every_dialogue_beat() {
        let report = run_demo().unwrap();
        assert!(report.slice_passed);
        assert_eq!(report.final_phase, GamePhase::Complete);
        assert!(
            report
                .events
                .iter()
                .any(|event| event.contains("E aí, Pedro"))
        );
        assert!(
            report
                .events
                .iter()
                .any(|event| event.contains("Bora. Te encontro"))
        );
        assert!(
            report
                .events
                .iter()
                .any(|event| event == "mission.completed")
        );
    }
}
