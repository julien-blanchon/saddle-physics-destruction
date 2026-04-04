use bevy::prelude::*;
use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};

use crate::{LabEntities, LabMetrics, inject_damage};

pub fn build() -> Scenario {
    Scenario::builder("destruction_effects")
        .description(
            "Damage the smoke crate, verify effect-hook messages fire, and capture the broken state with cue diagnostics populated.",
        )
        .then(Action::WaitFrames(30))
        .then(Action::Screenshot("destruction_effects_before".into()))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().smoke_crate;
            inject_damage(
                world,
                entity,
                Vec3::new(-7.0, 1.4, 0.9),
                Vec3::new(0.15, 0.0, -1.0),
                3.2,
                1.35,
                saddle_physics_destruction::FractureBias::Balanced,
            );
        })))
        .then(Action::WaitFrames(90))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "effect hooks incremented",
            |metrics| metrics.effect_triggers > 0,
        ))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "audio cue id captured",
            |metrics| !metrics.last_audio_cue.is_empty() && metrics.last_audio_cue != "-",
        ))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "particle cue id captured",
            |metrics| !metrics.last_particle_cue.is_empty() && metrics.last_particle_cue != "-",
        ))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "effect stage label recorded",
            |metrics| matches!(metrics.last_effect_stage.as_str(), "Started" | "Detached" | "FinalCollapse"),
        ))
        .then(inspect::log_resource::<LabMetrics>("effect metrics"))
        .then(assertions::log_summary("destruction_effects summary"))
        .then(Action::Screenshot("destruction_effects_after".into()))
        .build()
}
