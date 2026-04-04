use bevy::ecs::message::MessageCursor;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use super::*;
use crate::{
    ApplyDestructionDamage, CuboidAnchorPreset, CuboidFractureBuilder, Destructible,
    DestructionAssetHandle, DestructionConfig, DestructionEffectHooks, DestructionEffectStage,
    DestructionEffectTriggered, DestructionPlugin, Fragment, RootVisualMode, RuntimeFracture,
    ThinSurfaceAnchorPreset, ThinSurfaceFractureBuilder,
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

fn read_messages<T: Message + Clone>(app: &App, cursor: &mut MessageCursor<T>) -> Vec<T> {
    cursor
        .read(app.world().resource::<Messages<T>>())
        .cloned()
        .collect()
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

#[test]
fn configured_effect_hooks_emit_audio_and_particle_cues() {
    let mut app = test_app();
    let mut cursor = MessageCursor::<DestructionEffectTriggered>::default();
    let asset = CuboidFractureBuilder::new(Vec3::splat(2.0), UVec3::new(2, 1, 1)).build();
    let entity = spawn_destructible(
        &mut app,
        asset,
        "Hooked Root",
        Transform::default(),
        RootVisualMode::HideOnFirstDetach,
    );
    app.world_mut()
        .entity_mut(entity)
        .insert(DestructionEffectHooks {
            start_audio_cue: Some("destruction.start".into()),
            start_particle_cue: Some("dust_ring".into()),
            detach_audio_cue: Some("destruction.detach".into()),
            detach_particle_cue: Some("splinters".into()),
            final_audio_cue: Some("destruction.final".into()),
            final_particle_cue: Some("collapse".into()),
        });

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

    let messages = read_messages(&app, &mut cursor);
    assert!(
        messages
            .iter()
            .any(|message| message.stage == DestructionEffectStage::Started
                && message.audio_cue.as_deref() == Some("destruction.start")
                && message.particle_cue.as_deref() == Some("dust_ring"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.stage == DestructionEffectStage::Detached
                && message.audio_cue.as_deref() == Some("destruction.detach")
                && message.particle_cue.as_deref() == Some("splinters"))
    );
}

#[test]
fn runtime_fracture_component_generates_asset_handles() {
    let mut app = test_app();
    let entity = app
        .world_mut()
        .spawn((
            Name::new("Runtime Glass"),
            Destructible::default(),
            RuntimeFracture::thin_surface({
                let mut builder = ThinSurfaceFractureBuilder::new(Vec2::new(2.4, 1.8), 0.08, 9);
                builder.anchor_preset = ThinSurfaceAnchorPreset::Frame;
                builder
            }),
            Transform::default(),
        ))
        .id();

    app.update();

    let handle = app
        .world()
        .get::<DestructionAssetHandle>(entity)
        .expect("runtime fracture should create an asset handle");
    let asset = app
        .world()
        .resource::<Assets<FracturedAsset>>()
        .get(&handle.0)
        .expect("runtime fracture asset should be stored");

    assert_eq!(
        asset.metadata.generator,
        crate::FractureGenerator::ThinSurfaceVoronoi
    );
    assert_eq!(asset.support_chunk_count(), 9);
    assert!(
        app.world()
            .get::<crate::components::DestructionRuntime>(entity)
            .is_some()
    );
}

#[test]
fn changed_runtime_fracture_rebuilds_the_authored_asset() {
    let mut app = test_app();
    let entity = app
        .world_mut()
        .spawn((
            Name::new("Runtime Crate"),
            Destructible::default(),
            RuntimeFracture::cuboid(CuboidFractureBuilder::new(
                Vec3::splat(2.0),
                UVec3::new(2, 1, 1),
            )),
            Transform::default(),
        ))
        .id();

    app.update();

    app.world_mut()
        .entity_mut(entity)
        .insert(RuntimeFracture::cuboid({
            let mut builder = CuboidFractureBuilder::new(Vec3::splat(2.0), UVec3::new(3, 1, 1));
            builder.anchor_preset = CuboidAnchorPreset::None;
            builder
        }));
    app.update();

    let handle = app
        .world()
        .get::<DestructionAssetHandle>(entity)
        .expect("runtime fracture should rebuild an asset handle");
    let asset = app
        .world()
        .resource::<Assets<FracturedAsset>>()
        .get(&handle.0)
        .expect("rebuilt runtime fracture asset should exist");

    assert_eq!(asset.support_chunk_count(), 3);
    assert!(asset.fixed_support_chunks().is_empty());
}

#[cfg(feature = "avian3d")]
#[test]
fn avian_fragment_adapter_materializes_bodies_on_spawned_fragments() {
    let mut app = test_app();
    let asset = CuboidFractureBuilder::new(Vec3::splat(2.0), UVec3::new(2, 1, 1)).build();
    let entity = spawn_destructible(
        &mut app,
        asset,
        "Avian Root",
        Transform::default(),
        RootVisualMode::HideOnFirstDetach,
    );
    app.world_mut()
        .entity_mut(entity)
        .insert(crate::DestructionAvianFragments::default());

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

    let world = app.world_mut();
    let mut query = world.query::<(
        &Fragment,
        &avian3d::prelude::RigidBody,
        &avian3d::prelude::Mass,
        &avian3d::prelude::LinearVelocity,
        &avian3d::prelude::AngularVelocity,
    )>();
    let inserted = query.iter(world).collect::<Vec<_>>();
    assert!(
        !inserted.is_empty(),
        "expected avian adapter to insert rigid body data on spawned fragments"
    );
}
