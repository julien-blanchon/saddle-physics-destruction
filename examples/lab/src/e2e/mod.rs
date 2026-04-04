mod budget;
mod effects;
mod hierarchy;
mod lod;
mod smoke;
mod supports;

use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, scenario::Scenario};

pub struct DestructionLabE2EPlugin;

impl Plugin for DestructionLabE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(saddle_bevy_e2e::E2EPlugin);

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                saddle_bevy_e2e::init_scenario(app, scenario);
            } else {
                error!(
                    "[saddle_physics_destruction_lab:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
    let mut scenario_name = None;
    let mut handoff = false;

    for arg in args.iter().skip(1) {
        if arg == "--handoff" {
            handoff = true;
        } else if !arg.starts_with('-') && scenario_name.is_none() {
            scenario_name = Some(arg.clone());
        }
    }

    if !handoff {
        handoff = std::env::var("E2E_HANDOFF").is_ok_and(|value| value == "1" || value == "true");
    }

    (scenario_name, handoff)
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "destruction_smoke" => Some(smoke::build()),
        "destruction_effects" => Some(effects::build()),
        "destruction_supports" => Some(supports::build()),
        "destruction_hierarchy" => Some(hierarchy::build()),
        "destruction_lod" => Some(lod::build()),
        "destruction_budget" => Some(budget::build()),
        _ => None,
    }
}

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "destruction_smoke",
        "destruction_effects",
        "destruction_supports",
        "destruction_hierarchy",
        "destruction_lod",
        "destruction_budget",
    ]
}
