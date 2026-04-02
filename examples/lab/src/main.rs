#[cfg(feature = "e2e")]
mod e2e;

use bevy::prelude::*;
use saddle_physics_destruction::{
    ChunkGroupDetached, CuboidAnchorPreset, CuboidFractureBuilder, Destructible,
    DestructionAssetHandle, DestructionConfig, DestructionDiagnostics, DestructionPlugin,
    DestructionState, DestructionSystems, DestructionViewers, FinalDestructionOccurred,
    FracturedAsset, Fragment, FragmentLifetime, FragmentRenderData, FragmentSpawnData,
    MaterialHint, RootVisualMode, ThinSurfaceAnchorPreset, ThinSurfaceFractureBuilder,
    build_fragment_mesh,
};

#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
use bevy_brp_extras::BrpExtrasPlugin;
#[cfg(feature = "e2e")]
use saddle_physics_destruction::FractureBias;

#[derive(Component)]
struct LabCamera;

#[derive(Component)]
struct OverlayText;

#[derive(Component)]
struct LabFragmentMotion {
    linear_velocity: Vec3,
    angular_velocity: Vec3,
}

#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct LabEntities {
    pub smoke_crate: Entity,
    pub support_pillar: Entity,
    pub hierarchy_prop: Entity,
    pub lod_near: Entity,
    pub lod_far: Entity,
    pub budget_targets: Vec<Entity>,
}

#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct LabMetrics {
    pub smoke_detaches: u32,
    pub support_detaches: u32,
    pub hierarchy_detaches: u32,
    pub near_detaches: u32,
    pub far_detaches: u32,
    pub budget_detaches: u32,
    pub final_destructions: u32,
    pub peak_active_fragments: usize,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "destruction lab".into(),
            resolution: (1600, 960).into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.06, 0.065, 0.08)))
    .insert_resource(DestructionConfig {
        fragment_budget: 96,
        default_fragment_lifetime_secs: 5.5,
        fragment_fade_secs: 0.75,
        max_fragment_distance: 60.0,
        max_chunk_spawns_per_frame: 48,
        distance_lod: vec![
            saddle_physics_destruction::DistanceLodBand {
                max_distance: 20.0,
                strategy: saddle_physics_destruction::LodStrategy::Full,
            },
            saddle_physics_destruction::DistanceLodBand {
                max_distance: 30.0,
                strategy: saddle_physics_destruction::LodStrategy::Clustered {
                    minimum_leaf_count: 4,
                },
            },
            saddle_physics_destruction::DistanceLodBand {
                max_distance: f32::MAX,
                strategy: saddle_physics_destruction::LodStrategy::EventOnly,
            },
        ],
        ..default()
    })
    .init_resource::<LabMetrics>()
    .register_type::<LabEntities>()
    .register_type::<LabMetrics>()
    .add_plugins(DestructionPlugin::default());

    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    app.add_plugins(BrpExtrasPlugin::default());

    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::DestructionLabE2EPlugin);

    app.add_systems(Startup, setup_lab)
        .add_systems(
            Update,
            sync_viewers_from_camera.before(DestructionSystems::AccumulateDamage),
        )
        .add_systems(
            Update,
            (
                materialize_fragments,
                animate_fragments,
                fade_fragment_materials,
                update_peak_fragments,
                update_overlay,
            ),
        )
        .add_systems(
            Update,
            (track_detach_messages, track_final_destruction_messages)
                .after(DestructionSystems::ActivateFragments),
        );

    app.run();
}

fn setup_lab(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut fracture_assets: ResMut<Assets<FracturedAsset>>,
) {
    commands.spawn((
        Name::new("Lab Camera"),
        LabCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 9.0, 18.0).looking_at(Vec3::new(1.0, 2.0, -2.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Lab Sun"),
        DirectionalLight {
            illuminance: 34_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(12.0, 14.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Lab Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.18, 0.16),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::default(),
    ));
    commands.spawn((
        Name::new("Lab Overlay"),
        OverlayText,
        Text::new("destruction lab"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.0),
            left: px(12.0),
            ..default()
        },
    ));

    let smoke_crate_asset = {
        let mut builder = CuboidFractureBuilder::new(Vec3::new(2.4, 1.8, 2.2), UVec3::new(3, 2, 2));
        builder.anchor_preset = CuboidAnchorPreset::None;
        builder.coarse_groups = Some(UVec3::new(3, 1, 1));
        builder.material_hint = MaterialHint::Wood;
        builder.seed = 11;
        builder.build()
    };
    let smoke_crate = spawn_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut fracture_assets,
        "Smoke Crate",
        Transform::from_xyz(-7.0, 1.0, 1.0),
        smoke_crate_asset,
        RootVisualMode::HideOnFirstDetach,
    );

    let support_pillar_asset = {
        let mut builder = CuboidFractureBuilder::new(Vec3::new(2.0, 5.2, 2.0), UVec3::new(2, 6, 1));
        builder.anchor_preset = CuboidAnchorPreset::Bottom;
        builder.coarse_groups = Some(UVec3::new(2, 3, 1));
        builder.material_hint = MaterialHint::Stone;
        builder.seed = 44;
        builder.build()
    };
    let support_pillar = spawn_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut fracture_assets,
        "Support Pillar",
        Transform::from_xyz(-2.0, 2.6, 0.0),
        support_pillar_asset,
        RootVisualMode::HideWhenBroken,
    );

    let hierarchy_asset = {
        let mut builder = CuboidFractureBuilder::new(Vec3::new(3.0, 4.8, 2.4), UVec3::new(4, 4, 2));
        builder.anchor_preset = CuboidAnchorPreset::Bottom;
        builder.coarse_groups = Some(UVec3::new(2, 4, 1));
        builder.material_hint = MaterialHint::Concrete;
        builder.seed = 77;
        builder.build()
    };
    let hierarchy_prop = spawn_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut fracture_assets,
        "Hierarchy Prop",
        Transform::from_xyz(3.2, 2.5, 0.0),
        hierarchy_asset,
        RootVisualMode::HideOnFirstDetach,
    );

    let near_asset = {
        let mut builder = CuboidFractureBuilder::new(Vec3::new(1.8, 1.8, 1.8), UVec3::new(2, 2, 2));
        builder.anchor_preset = CuboidAnchorPreset::None;
        builder.coarse_groups = Some(UVec3::new(2, 1, 1));
        builder.material_hint = MaterialHint::Metal;
        builder.seed = 501;
        builder.build()
    };
    let lod_near = spawn_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut fracture_assets,
        "LOD Near",
        Transform::from_xyz(8.5, 1.0, 4.0),
        near_asset,
        RootVisualMode::HideOnFirstDetach,
    );

    let far_asset = {
        let mut builder = CuboidFractureBuilder::new(Vec3::new(2.2, 2.2, 2.2), UVec3::new(3, 2, 2));
        builder.anchor_preset = CuboidAnchorPreset::None;
        builder.coarse_groups = Some(UVec3::new(3, 1, 1));
        builder.material_hint = MaterialHint::Concrete;
        builder.seed = 777;
        builder.build()
    };
    let lod_far = spawn_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut fracture_assets,
        "LOD Far",
        Transform::from_xyz(8.5, 1.2, -22.0),
        far_asset,
        RootVisualMode::HideOnFirstDetach,
    );

    let panel_asset = {
        let mut builder = ThinSurfaceFractureBuilder::new(Vec2::new(4.0, 2.4), 0.05, 18);
        builder.anchor_preset = ThinSurfaceAnchorPreset::Frame;
        builder.material_hint = MaterialHint::Glass;
        builder.seed = 1024;
        builder.build()
    };
    let _panel = spawn_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut fracture_assets,
        "Thin Surface Panel",
        Transform::from_xyz(-9.0, 1.7, -8.0),
        panel_asset,
        RootVisualMode::HideOnFirstDetach,
    );

    let mut budget_targets = Vec::new();
    for row in 0..2 {
        for col in 0..4 {
            let mut builder =
                CuboidFractureBuilder::new(Vec3::new(1.3, 1.3, 1.3), UVec3::new(2, 2, 2));
            builder.anchor_preset = CuboidAnchorPreset::None;
            builder.coarse_groups = Some(UVec3::new(2, 1, 1));
            builder.material_hint = MaterialHint::Concrete;
            builder.seed = 900 + (row * 4 + col) as u64;
            let asset = builder.build();
            budget_targets.push(spawn_root(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut fracture_assets,
                &format!("Budget Target {row}-{col}"),
                Transform::from_xyz(3.5 + col as f32 * 2.0, 0.8, -10.0 - row as f32 * 2.0),
                asset,
                RootVisualMode::HideOnFirstDetach,
            ));
        }
    }

    commands.insert_resource(LabEntities {
        smoke_crate,
        support_pillar,
        hierarchy_prop,
        lod_near,
        lod_far,
        budget_targets,
    });
}

fn spawn_root(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    fracture_assets: &mut Assets<FracturedAsset>,
    name: &str,
    transform: Transform,
    asset: FracturedAsset,
    visual_mode: RootVisualMode,
) -> Entity {
    let preview_render = if asset.root_chunks.len() == 1 {
        let root_chunk = asset.chunk(asset.root_chunks[0]);
        if !root_chunk.support_node {
            root_chunk.render.clone()
        } else {
            FragmentRenderData::Cuboid {
                size: asset.bounds,
                interior_material_slot: 0,
            }
        }
    } else {
        FragmentRenderData::Cuboid {
            size: asset.bounds,
            interior_material_slot: 0,
        }
    };
    let material_hint = asset
        .root_chunks
        .first()
        .map(|chunk_id| asset.chunk(*chunk_id).material_hint)
        .unwrap_or(MaterialHint::Generic);
    let handle = fracture_assets.add(asset);

    commands
        .spawn((
            Name::new(name.to_string()),
            Destructible { visual_mode },
            DestructionAssetHandle(handle),
            Mesh3d(meshes.add(build_fragment_mesh(&preview_render))),
            MeshMaterial3d(materials.add(material_for_hint(material_hint, 1.0))),
            transform,
        ))
        .id()
}

fn sync_viewers_from_camera(
    mut viewers: ResMut<DestructionViewers>,
    cameras: Query<&Transform, With<LabCamera>>,
) {
    viewers.positions = cameras
        .iter()
        .map(|transform| transform.translation)
        .collect();
}

fn materialize_fragments(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    fragments: Query<(Entity, &Fragment, &FragmentSpawnData), Added<Fragment>>,
) {
    for (entity, fragment, spawn_data) in &fragments {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(build_fragment_mesh(&spawn_data.render))),
            MeshMaterial3d(materials.add(material_for_hint(fragment.material_hint, 1.0))),
            LabFragmentMotion {
                linear_velocity: spawn_data.initial_velocity.linear,
                angular_velocity: spawn_data.initial_velocity.angular,
            },
        ));
    }
}

fn animate_fragments(
    time: Res<Time>,
    mut fragments: Query<(&mut Transform, &mut LabFragmentMotion)>,
) {
    let delta = time.delta_secs();
    for (mut transform, mut motion) in &mut fragments {
        motion.linear_velocity += Vec3::new(0.0, -8.2, 0.0) * delta;
        transform.translation += motion.linear_velocity * delta;
        transform.rotate(Quat::from_euler(
            EulerRot::XYZ,
            motion.angular_velocity.x * delta,
            motion.angular_velocity.y * delta,
            motion.angular_velocity.z * delta,
        ));
    }
}

fn fade_fragment_materials(
    mut materials: ResMut<Assets<StandardMaterial>>,
    fragments: Query<(
        &Fragment,
        &FragmentLifetime,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for (fragment, lifetime, material_handle) in &fragments {
        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };
        *material = material_for_hint(fragment.material_hint, lifetime.normalized_alpha);
    }
}

fn track_detach_messages(
    entities: Res<LabEntities>,
    mut metrics: ResMut<LabMetrics>,
    mut reader: MessageReader<ChunkGroupDetached>,
) {
    for message in reader.read() {
        if message.source == entities.smoke_crate {
            metrics.smoke_detaches += 1;
        } else if message.source == entities.support_pillar {
            metrics.support_detaches += 1;
        } else if message.source == entities.hierarchy_prop {
            metrics.hierarchy_detaches += 1;
        } else if message.source == entities.lod_near {
            metrics.near_detaches += 1;
        } else if message.source == entities.lod_far {
            metrics.far_detaches += 1;
        } else if entities.budget_targets.contains(&message.source) {
            metrics.budget_detaches += 1;
        }
    }
}

fn track_final_destruction_messages(
    mut metrics: ResMut<LabMetrics>,
    mut reader: MessageReader<FinalDestructionOccurred>,
) {
    metrics.final_destructions += reader.read().count() as u32;
}

fn update_peak_fragments(
    diagnostics: Res<DestructionDiagnostics>,
    mut metrics: ResMut<LabMetrics>,
) {
    metrics.peak_active_fragments = metrics
        .peak_active_fragments
        .max(diagnostics.active_fragments);
}

fn update_overlay(
    entities: Res<LabEntities>,
    metrics: Res<LabMetrics>,
    diagnostics: Res<DestructionDiagnostics>,
    states: Query<(&Name, &DestructionState), With<Destructible>>,
    mut text: Single<&mut Text, With<OverlayText>>,
) {
    let mut lines = vec![
        format!("active fragments: {}", diagnostics.active_fragments),
        format!("peak fragments: {}", metrics.peak_active_fragments),
        format!(
            "detach messages smoke/support/hierarchy/near/far/budget: {}/{}/{}/{}/{}/{}",
            metrics.smoke_detaches,
            metrics.support_detaches,
            metrics.hierarchy_detaches,
            metrics.near_detaches,
            metrics.far_detaches,
            metrics.budget_detaches
        ),
        format!("final destructions: {}", metrics.final_destructions),
        format!("budget target count: {}", entities.budget_targets.len()),
    ];

    for (name, state) in &states {
        lines.push(format!(
            "{} -> dmg {:.2} detached {} broken {}",
            name.as_str(),
            state.normalized_damage,
            state.detached_chunks,
            state.broken
        ));
    }

    text.0 = lines.join("\n");
}

fn material_for_hint(material_hint: MaterialHint, alpha: f32) -> StandardMaterial {
    match material_hint {
        MaterialHint::Glass => StandardMaterial {
            base_color: Color::srgba(0.66, 0.82, 0.94, alpha * 0.45),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.05,
            reflectance: 0.7,
            ..default()
        },
        MaterialHint::Wood => StandardMaterial {
            base_color: Color::srgba(0.57, 0.38, 0.22, alpha),
            perceptual_roughness: 0.82,
            ..default()
        },
        MaterialHint::Stone | MaterialHint::Concrete => StandardMaterial {
            base_color: Color::srgba(0.68, 0.69, 0.73, alpha),
            perceptual_roughness: 0.94,
            ..default()
        },
        MaterialHint::Metal => StandardMaterial {
            base_color: Color::srgba(0.64, 0.67, 0.73, alpha),
            metallic: 0.82,
            perceptual_roughness: 0.24,
            ..default()
        },
        MaterialHint::Ceramic => StandardMaterial {
            base_color: Color::srgba(0.83, 0.8, 0.76, alpha),
            perceptual_roughness: 0.32,
            ..default()
        },
        MaterialHint::Generic => StandardMaterial {
            base_color: Color::srgba(0.78, 0.55, 0.35, alpha),
            perceptual_roughness: 0.65,
            ..default()
        },
    }
}

#[cfg(feature = "e2e")]
pub(crate) fn inject_damage(
    world: &mut World,
    entity: Entity,
    origin: Vec3,
    direction: Vec3,
    magnitude: f32,
    radius: f32,
    fracture_bias: FractureBias,
) {
    world
        .resource_mut::<Messages<saddle_physics_destruction::ApplyDestructionDamage>>()
        .write(saddle_physics_destruction::ApplyDestructionDamage {
            target: entity,
            origin,
            direction,
            magnitude,
            radius,
            kind: saddle_physics_destruction::DamageKind::Radial,
            falloff: saddle_physics_destruction::FalloffCurve::SmoothStep,
            fracture_bias,
        });
}

#[cfg(feature = "e2e")]
pub(crate) fn fragment_count_for_source(world: &mut World, source: Entity) -> usize {
    let mut query = world.query::<&Fragment>();
    query
        .iter(world)
        .filter(|fragment| fragment.source == source)
        .count()
}

#[cfg(feature = "e2e")]
pub(crate) fn leaf_fragment_count_for_source(world: &mut World, source: Entity) -> usize {
    let mut query = world.query::<&Fragment>();
    query
        .iter(world)
        .filter(|fragment| fragment.source == source && fragment.chunk_count == 1)
        .count()
}
