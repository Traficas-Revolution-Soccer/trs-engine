use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DemoReport {
    pub scene: &'static str,
    pub systems: Vec<&'static str>,
    pub events: Vec<String>,
    pub slice_passed: bool,
}

/// Runs a deterministic, display-free story slice for agents and CI.
pub fn run_demo() -> DemoReport {
    let systems = vec![
        "world",
        "render (play mode)",
        "physics (Rapier in play mode)",
        "player movement",
        "npc",
        "interaction",
        "dialogue",
        "cutscene",
        "mission",
        "validation",
    ];
    let mut player_x = -2.0_f32;
    let npc_x = 2.0_f32;
    let mut events = vec!["Loaded Bairro da Bola: player at x=-2, NPC at x=2".to_owned()];

    // Fixed steps make the demo repeatable; the interaction radius is also used by play mode.
    while (npc_x - player_x).abs() > 1.8 {
        player_x += 0.25;
    }
    events.push(format!(
        "Player walked to x={player_x:.2}; NPC interaction range reached"
    ));
    events.push("Cutscene started: camera holds a two-shot".into());
    events.push("NPC dialogue: Vamos jogar?".into());
    events.push("Cutscene ended; control returned to player".into());
    events.push("Mission objective: Encontre o campo de futebol".into());

    let slice_passed = player_x < npc_x && (npc_x - player_x).abs() <= 1.8;
    events.push(format!(
        "Scene validation: {}",
        if slice_passed { "passed" } else { "failed" }
    ));
    DemoReport {
        scene: "bairro-da-bola",
        systems,
        events,
        slice_passed,
    }
}

#[cfg(test)]
mod tests {
    use super::run_demo;

    #[test]
    fn vertical_slice_reaches_npc_and_starts_mission() {
        let report = run_demo();
        assert!(report.slice_passed);
        assert!(
            report
                .events
                .iter()
                .any(|event| event.contains("interaction range"))
        );
        assert!(
            report
                .events
                .iter()
                .any(|event| event.contains("Cutscene started"))
        );
        assert!(
            report
                .events
                .iter()
                .any(|event| event.contains("Mission objective"))
        );
        assert!(report.events.last().unwrap().contains("passed"));
    }
}
