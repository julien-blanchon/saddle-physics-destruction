use bevy::prelude::*;
use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};

use crate::{LabEntities, LabMetrics, inject_damage};

pub fn build() -> Scenario {
    Scenario::builder("destruction_budget")
        .description("Hammer the budget targets repeatedly and verify the fragment budget stays bounded while cleanup trims old debris.")
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut config = world.resource_mut::<saddle_physics_destruction::DestructionConfig>();
            config.fragment_budget = 6;
        })))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let targets = world.resource::<LabEntities>().budget_targets.clone();
            for (index, target) in targets.into_iter().enumerate() {
                inject_damage(
                    world,
                    target,
                    Vec3::new(
                        3.5 + (index % 4) as f32 * 2.0,
                        1.0,
                        -10.0 - (index / 4) as f32 * 2.0,
                    ),
                    Vec3::new(0.0, 0.0, -1.0),
                    2.6,
                    1.0,
                    saddle_physics_destruction::FractureBias::Balanced,
                );
            }
        })))
        .then(Action::WaitFrames(40))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let targets = world.resource::<LabEntities>().budget_targets.clone();
            for (index, target) in targets.into_iter().enumerate() {
                inject_damage(
                    world,
                    target,
                    Vec3::new(
                        3.5 + (index % 4) as f32 * 2.0,
                        1.2,
                        -10.0 - (index / 4) as f32 * 2.0,
                    ),
                    Vec3::new(0.2, 0.0, -1.0),
                    2.7,
                    1.0,
                    saddle_physics_destruction::FractureBias::Balanced,
                );
            }
        })))
        .then(Action::WaitFrames(140))
        .then(assertions::resource_satisfies::<saddle_physics_destruction::DestructionDiagnostics>(
            "active fragments stay within configured budget",
            |diagnostics| diagnostics.active_fragments <= 6 && diagnostics.total_budget_trims > 0,
        ))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "budget detach activity recorded",
            |metrics| metrics.budget_detaches > 0 && metrics.peak_active_fragments >= 6,
        ))
        .then(inspect::log_resource::<saddle_physics_destruction::DestructionDiagnostics>(
            "budget diagnostics",
        ))
        .then(inspect::log_resource::<LabMetrics>("budget metrics"))
        .then(assertions::log_summary("destruction_budget summary"))
        .then(Action::Screenshot("destruction_budget_after".into()))
        .build()
}
