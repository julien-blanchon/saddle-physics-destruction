use bevy::prelude::*;
use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};

use crate::{LabEntities, LabMetrics, fragment_count_for_source, inject_damage};

pub fn build() -> Scenario {
    Scenario::builder("destruction_smoke")
        .description("Break the smoke crate, verify fragment activation, and capture before/after screenshots.")
        .then(Action::WaitFrames(30))
        .then(Action::Screenshot("destruction_smoke_before".into()))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().smoke_crate;
            inject_damage(
                world,
                entity,
                Vec3::new(-7.0, 1.6, 1.2),
                Vec3::new(0.0, 0.0, -1.0),
                3.0,
                1.3,
                saddle_physics_destruction::FractureBias::Balanced,
            );
        })))
        .then(Action::WaitFrames(80))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().smoke_crate;
            let fragment_count = fragment_count_for_source(world, entity);
            assert!(
                fragment_count > 0,
                "expected smoke crate to emit fragments, found {fragment_count}"
            );
        })))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "smoke detach messages recorded",
            |metrics| metrics.smoke_detaches > 0,
        ))
        .then(assertions::custom("smoke crate hidden after detach", |world| {
            let entity = world.resource::<LabEntities>().smoke_crate;
            world
                .get::<Visibility>(entity)
                .is_some_and(|visibility| *visibility == Visibility::Hidden)
        }))
        .then(inspect::log_resource::<LabMetrics>("smoke metrics"))
        .then(assertions::log_summary("destruction_smoke summary"))
        .then(Action::Screenshot("destruction_smoke_after".into()))
        .build()
}
