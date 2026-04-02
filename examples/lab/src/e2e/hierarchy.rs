use bevy::prelude::*;
use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};

use crate::{LabCamera, LabEntities, LabMetrics, inject_damage, leaf_fragment_count_for_source};

#[derive(Resource, Default)]
struct HierarchySnapshot {
    coarse_leaf_fragments: usize,
}

pub fn build() -> Scenario {
    Scenario::builder("destruction_hierarchy")
        .description("Drive a coarse pass and a fine pass on the hierarchy prop, verifying staged fracture detail increases over time.")
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let camera_position = Vec3::new(0.0, 10.0, 26.0);
            let mut cameras = world.query_filtered::<&mut Transform, With<LabCamera>>();
            for mut transform in cameras.iter_mut(world) {
                *transform =
                    Transform::from_translation(camera_position).looking_at(Vec3::new(3.2, 2.5, 0.0), Vec3::Y);
            }
            world.resource_mut::<saddle_physics_destruction::DestructionViewers>().positions = vec![camera_position];
        })))
        .then(Action::WaitFrames(5))
        .then(Action::Screenshot("destruction_hierarchy_before".into()))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().hierarchy_prop;
            inject_damage(
                world,
                entity,
                Vec3::new(2.4, 1.0, 0.5),
                Vec3::new(1.0, 0.0, 0.0),
                3.6,
                1.5,
                saddle_physics_destruction::FractureBias::Coarse,
            );
        })))
        .then(Action::WaitFrames(25))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().hierarchy_prop;
            inject_damage(
                world,
                entity,
                Vec3::new(4.0, 1.0, -0.4),
                Vec3::new(-1.0, 0.0, 0.0),
                3.6,
                1.5,
                saddle_physics_destruction::FractureBias::Coarse,
            );
        })))
        .then(Action::WaitFrames(70))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().hierarchy_prop;
            let leaf_count = leaf_fragment_count_for_source(world, entity);
            assert!(
                (1..=4).contains(&leaf_count),
                "expected hierarchy coarse pass to spawn a bounded first fracture, found {leaf_count} leaf fragments"
            );
            world.insert_resource(HierarchySnapshot {
                coarse_leaf_fragments: leaf_count,
            });
        })))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().hierarchy_prop;
            inject_damage(
                world,
                entity,
                Vec3::new(3.3, 2.8, -0.2),
                Vec3::new(0.0, 0.0, -1.0),
                3.5,
                1.35,
                saddle_physics_destruction::FractureBias::Fine,
            );
        })))
        .then(Action::WaitFrames(80))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let entity = world.resource::<LabEntities>().hierarchy_prop;
            let coarse_leaf_fragments = world
                .get_resource::<HierarchySnapshot>()
                .map(|snapshot| snapshot.coarse_leaf_fragments)
                .unwrap_or_default();
            let leaf_count = leaf_fragment_count_for_source(world, entity);
            assert!(
                leaf_count > coarse_leaf_fragments,
                "expected fine pass to increase leaf fragment detail beyond {coarse_leaf_fragments}, found {leaf_count}"
            );
        })))
        .then(assertions::resource_satisfies::<LabMetrics>(
            "hierarchy detach messages recorded",
            |metrics| metrics.hierarchy_detaches > 0,
        ))
        .then(inspect::log_world_summary("hierarchy world"))
        .then(assertions::log_summary("destruction_hierarchy summary"))
        .then(Action::Screenshot("destruction_hierarchy_after".into()))
        .build()
}
