use bevy::prelude::*;
use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};

use crate::{LabEntities, LabMetrics, fragment_count_for_source, inject_damage};

pub fn build() -> Scenario {
    Scenario::builder("destruction_lod")
        .description("Hit near and far LOD targets, verify near spawns fragments while the far target degrades to event-only handling.")
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let near = world.resource::<LabEntities>().lod_near;
            let far = world.resource::<LabEntities>().lod_far;
            inject_damage(
                world,
                near,
                Vec3::new(8.4, 1.5, 4.2),
                Vec3::new(0.0, 0.0, -1.0),
                2.8,
                1.1,
                saddle_physics_destruction::FractureBias::Balanced,
            );
            inject_damage(
                world,
                far,
                Vec3::new(8.4, 1.6, -22.0),
                Vec3::new(0.0, 0.0, -1.0),
                3.4,
                1.35,
                saddle_physics_destruction::FractureBias::Balanced,
            );
        })))
        .then(Action::WaitFrames(24))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let far = world.resource::<LabEntities>().lod_far;
            inject_damage(
                world,
                far,
                Vec3::new(8.8, 1.4, -22.2),
                Vec3::new(-0.2, 0.0, -1.0),
                3.2,
                1.2,
                saddle_physics_destruction::FractureBias::Balanced,
            );
        })))
        .then(Action::WaitFrames(72))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().lod_near;
            let fragment_count = fragment_count_for_source(world, entity);
            assert!(
                fragment_count > 0,
                "expected near LOD target to spawn fragments, found {fragment_count}"
            );
        })))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().lod_far;
            let fragment_count = fragment_count_for_source(world, entity);
            assert!(
                fragment_count == 0,
                "expected far LOD target to remain event-only, found {fragment_count} fragment entities"
            );
        })))
        .then(assertions::custom("far target detached without fragment spawn", |world| {
            let entity = world.resource::<LabEntities>().lod_far;
            world
                .get::<saddle_physics_destruction::DestructionState>(entity)
                .is_some_and(|state| state.detached_chunks > 0)
        }))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "far detach events still recorded",
            |metrics| metrics.far_detaches > 0 && metrics.near_detaches > 0,
        ))
        .then(inspect::log_resource::<LabMetrics>("lod metrics"))
        .then(assertions::log_summary("destruction_lod summary"))
        .then(Action::Screenshot("destruction_lod_after".into()))
        .build()
}
