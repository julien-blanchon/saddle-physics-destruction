use bevy::prelude::*;
use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};

use crate::{LabEntities, LabMetrics, fragment_count_for_source, inject_damage};

pub fn build() -> Scenario {
    Scenario::builder("destruction_supports")
        .description(
            "Damage the anchored pillar base and verify the unsupported upper section collapses.",
        )
        .then(Action::WaitFrames(30))
        .then(Action::Screenshot("destruction_supports_before".into()))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().support_pillar;
            inject_damage(
                world,
                entity,
                Vec3::new(-2.8, 0.85, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                3.0,
                1.25,
                saddle_physics_destruction::FractureBias::Balanced,
            );
        })))
        .then(Action::WaitFrames(25))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().support_pillar;
            inject_damage(
                world,
                entity,
                Vec3::new(-1.2, 0.9, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                3.0,
                1.25,
                saddle_physics_destruction::FractureBias::Balanced,
            );
        })))
        .then(Action::WaitFrames(90))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().support_pillar;
            let fragment_count = fragment_count_for_source(world, entity);
            assert!(
                fragment_count > 0,
                "expected pillar to spawn fragments, found {fragment_count}"
            );
        })))
        .then(assertions::custom(
            "pillar detached chunks increased",
            |world| {
                let entity = world.resource::<LabEntities>().support_pillar;
                world
                    .get::<saddle_physics_destruction::DestructionState>(entity)
                    .is_some_and(|state| state.detached_chunks >= 2)
            },
        ))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "support detach messages recorded",
            |metrics| metrics.support_detaches > 0,
        ))
        .then(inspect::log_resource::<LabMetrics>("support metrics"))
        .then(assertions::log_summary("destruction_supports summary"))
        .then(Action::Screenshot("destruction_supports_after".into()))
        .build()
}
