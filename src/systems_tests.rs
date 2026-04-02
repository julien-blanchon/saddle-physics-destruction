use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use super::*;
use crate::{
    ApplyDestructionDamage, CuboidAnchorPreset, CuboidFractureBuilder, Destructible,
    DestructionAssetHandle, DestructionConfig, DestructionPlugin, Fragment, RootVisualMode,
};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Activate;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Deactivate;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(TransformPlugin)
        .add_plugins(DestructionPlugin::new(Activate, Deactivate, Update));
    app
}

fn spawn_destructible(
    app: &mut App,
    asset: FracturedAsset,
    name: &str,
    transform: Transform,
    visual_mode: RootVisualMode,
) -> Entity {
    let handle = app
        .world_mut()
        .resource_mut::<Assets<FracturedAsset>>()
        .add(asset);
    app.world_mut()
        .spawn((
            Name::new(name.to_string()),
            Destructible { visual_mode },
            DestructionAssetHandle(handle),
            transform,
        ))
        .id()
}

#[test]
fn plugin_builds_and_consumes_messages() {
    let mut app = test_app();
    let asset = CuboidFractureBuilder::new(Vec3::splat(2.0), UVec3::new(2, 1, 1)).build();
    let entity = spawn_destructible(
        &mut app,
        asset,
        "Test Root",
        Transform::default(),
        RootVisualMode::HideOnFirstDetach,
    );

    app.update();
    app.world_mut()
        .resource_mut::<Messages<ApplyDestructionDamage>>()
        .write(ApplyDestructionDamage::radial(entity, Vec3::ZERO, 4.0, 2.0));
    app.update();

    let state = app.world().get::<DestructionState>(entity).unwrap();
    assert!(state.normalized_damage > 0.0);
}

#[test]
fn world_space_damage_hits_translated_roots() {
    let mut app = test_app();
    let asset = CuboidFractureBuilder::new(Vec3::splat(2.0), UVec3::new(2, 1, 1)).build();
    let entity = spawn_destructible(
        &mut app,
        asset,
        "Translated Root",
        Transform::from_xyz(8.0, 0.0, -3.0),
        RootVisualMode::HideOnFirstDetach,
    );

    app.update();
    app.world_mut()
        .resource_mut::<Messages<ApplyDestructionDamage>>()
        .write(ApplyDestructionDamage::radial(
            entity,
            Vec3::new(8.0, 0.0, -3.0),
            4.0,
            2.0,
        ));
    app.update();

    let state = app.world().get::<DestructionState>(entity).unwrap();
    assert!(
        state.normalized_damage > 0.0,
        "expected translated destructible to receive world-space damage"
    );
}

#[test]
fn free_floating_roots_keep_one_connected_component_and_hide_on_detach() {
    let mut app = test_app();
    let mut builder = CuboidFractureBuilder::new(Vec3::splat(2.0), UVec3::new(2, 1, 1));
    builder.anchor_preset = CuboidAnchorPreset::None;
    let mut asset = builder.build();
    for chunk_id in asset.support_chunks.clone() {
        asset.chunks[chunk_id.index()].damage_threshold = 1000.0;
    }
    asset.bonds[0].health = 0.01;

    let entity = spawn_destructible(
        &mut app,
        asset,
        "Floating Root",
        Transform::default(),
        RootVisualMode::HideOnFirstDetach,
    );

    app.update();
    app.world_mut()
        .resource_mut::<Messages<ApplyDestructionDamage>>()
        .write(ApplyDestructionDamage::radial(entity, Vec3::ZERO, 2.5, 4.0));
    app.update();

    let runtime = app
        .world()
        .get::<crate::components::DestructionRuntime>(entity)
        .unwrap();
    assert!(
        runtime.bond_health[0] <= 0.0,
        "expected support-breaking hit to consume the shared bond, found {:?}",
        runtime.bond_health
    );
    assert_eq!(
        runtime
            .detached_chunks
            .iter()
            .filter(|detached| **detached)
            .count(),
        1,
        "expected support evaluation to detach exactly one floating component"
    );
    let state = app.world().get::<DestructionState>(entity).unwrap();
    assert_eq!(state.detached_chunks, 1);
    assert_eq!(state.active_fragments, 1);
    assert_eq!(
        app.world().get::<Visibility>(entity),
        Some(&Visibility::Hidden),
        "support-only detachments should still hide HideOnFirstDetach roots"
    );
}

#[test]
fn activation_throttle_preserves_backlog_across_frames() {
    let mut app = test_app();
    app.insert_resource(DestructionConfig {
        max_chunk_spawns_per_frame: 1,
        enable_support_evaluation: false,
        ..default()
    });

    let mut builder = CuboidFractureBuilder::new(Vec3::splat(2.0), UVec3::new(2, 1, 1));
    builder.anchor_preset = CuboidAnchorPreset::None;
    let mut asset = builder.build();
    for chunk_id in asset.support_chunks.clone() {
        asset.chunks[chunk_id.index()].damage_threshold = 0.1;
    }

    let entity = spawn_destructible(
        &mut app,
        asset,
        "Throttled Root",
        Transform::default(),
        RootVisualMode::HideOnFirstDetach,
    );

    app.update();
    app.world_mut()
        .resource_mut::<Messages<ApplyDestructionDamage>>()
        .write(ApplyDestructionDamage::radial(
            entity,
            Vec3::ZERO,
            10.0,
            3.0,
        ));
    app.update();

    let diagnostics = app.world().resource::<crate::DestructionDiagnostics>();
    assert_eq!(diagnostics.total_detached_groups, 2);
    assert_eq!(diagnostics.pending_groups, 1);

    let state = app.world().get::<DestructionState>(entity).unwrap();
    assert_eq!(state.active_fragments, 1);

    let runtime = app
        .world()
        .get::<crate::components::DestructionRuntime>(entity)
        .unwrap();
    assert_eq!(runtime.pending_groups.len(), 1);

    app.update();

    let diagnostics = app.world().resource::<crate::DestructionDiagnostics>();
    assert_eq!(diagnostics.pending_groups, 0);

    let state = app.world().get::<DestructionState>(entity).unwrap();
    assert_eq!(state.active_fragments, 2);

    let fragments = {
        let world = app.world_mut();
        let mut query = world.query::<&Fragment>();
        query.iter(world).count()
    };
    assert_eq!(fragments, 2);
}
