use bevy::prelude::*;
use saddle_physics_saddle_physics_destruction::{
    ApplyDestructionDamage, Destructible, DestructionAssetHandle, DestructionDiagnostics,
    DestructionPlugin, DestructionState, DestructionViewers, FractureBias, FracturedAsset,
    Fragment, FragmentLifetime, FragmentRenderData, FragmentSpawnData, MaterialHint,
    RootVisualMode, build_fragment_mesh,
};

#[derive(Component)]
pub struct DemoFragmentMotion {
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
}

#[derive(Component)]
pub struct DiagnosticsText;

pub fn add_base_plugins(app: &mut App) {
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "destruction example".into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(DestructionPlugin::default())
    .add_systems(Startup, setup_world)
    .add_systems(
        Update,
        (
            sync_viewers_from_camera,
            materialize_fragments,
            animate_fragments,
            fade_fragment_materials,
            update_diagnostics_text,
        ),
    );
}

pub fn spawn_preview_root(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_handle: Handle<FracturedAsset>,
    asset: &FracturedAsset,
    label: &str,
    transform: Transform,
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
    let material_hint = dominant_material(asset);

    commands
        .spawn((
            Name::new(label.to_string()),
            Destructible { visual_mode },
            DestructionAssetHandle(asset_handle),
            Mesh3d(meshes.add(build_fragment_mesh(&preview_render))),
            MeshMaterial3d(materials.add(material_for_hint(material_hint, 1.0))),
            transform,
        ))
        .id()
}

pub fn emit_damage(
    writer: &mut MessageWriter<ApplyDestructionDamage>,
    entity: Entity,
    origin: Vec3,
    direction: Vec3,
    magnitude: f32,
    radius: f32,
    fracture_bias: FractureBias,
) {
    writer.write(ApplyDestructionDamage {
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

pub fn material_for_hint(material_hint: MaterialHint, alpha: f32) -> StandardMaterial {
    match material_hint {
        MaterialHint::Glass => StandardMaterial {
            base_color: Color::srgba(0.65, 0.82, 0.94, alpha * 0.45),
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::new(0.02, 0.05, 0.08, 1.0),
            perceptual_roughness: 0.04,
            reflectance: 0.65,
            ..default()
        },
        MaterialHint::Wood => StandardMaterial {
            base_color: Color::srgba(0.58, 0.39, 0.24, alpha),
            perceptual_roughness: 0.8,
            ..default()
        },
        MaterialHint::Stone | MaterialHint::Concrete => StandardMaterial {
            base_color: Color::srgba(0.67, 0.68, 0.72, alpha),
            perceptual_roughness: 0.92,
            ..default()
        },
        MaterialHint::Metal => StandardMaterial {
            base_color: Color::srgba(0.65, 0.68, 0.72, alpha),
            metallic: 0.8,
            perceptual_roughness: 0.22,
            ..default()
        },
        MaterialHint::Ceramic => StandardMaterial {
            base_color: Color::srgba(0.82, 0.8, 0.76, alpha),
            perceptual_roughness: 0.34,
            ..default()
        },
        MaterialHint::Generic => StandardMaterial {
            base_color: Color::srgba(0.78, 0.54, 0.32, alpha),
            perceptual_roughness: 0.66,
            ..default()
        },
    }
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.1)));

    commands.spawn((
        Name::new("Example Camera"),
        Camera3d::default(),
        Transform::from_xyz(9.0, 6.4, 10.5).looking_at(Vec3::new(0.0, 1.8, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Sun Light"),
        DirectionalLight {
            illuminance: 26_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(8.0, 14.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.14, 0.18, 0.14),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::default(),
    ));
    commands.spawn((
        Name::new("Diagnostics"),
        DiagnosticsText,
        Text::new("destruction"),
        Node {
            position_type: PositionType::Absolute,
            top: px(14),
            left: px(14),
            ..default()
        },
    ));
}

fn sync_viewers_from_camera(
    mut viewers: ResMut<DestructionViewers>,
    cameras: Query<&Transform, With<Camera3d>>,
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
            DemoFragmentMotion {
                linear_velocity: spawn_data.initial_velocity.linear,
                angular_velocity: spawn_data.initial_velocity.angular,
            },
        ));
    }
}

fn animate_fragments(
    time: Res<Time>,
    mut fragments: Query<(&mut Transform, &mut DemoFragmentMotion)>,
) {
    let delta = time.delta_secs();
    for (mut transform, mut motion) in &mut fragments {
        motion.linear_velocity += Vec3::new(0.0, -7.5, 0.0) * delta;
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

fn update_diagnostics_text(
    diagnostics: Res<DestructionDiagnostics>,
    destructibles: Query<(&Name, &DestructionState), With<Destructible>>,
    mut text: Single<&mut Text, With<DiagnosticsText>>,
) {
    let mut lines = vec![
        format!("fragments: {}", diagnostics.active_fragments),
        format!("pending groups: {}", diagnostics.pending_groups),
        format!("support evals: {}", diagnostics.total_support_evaluations),
    ];
    for (name, state) in &destructibles {
        lines.push(format!(
            "{} dmg {:.2} detached {} broken {}",
            name.as_str(),
            state.normalized_damage,
            state.detached_chunks,
            state.broken
        ));
    }
    text.0 = lines.join("\n");
}

fn dominant_material(asset: &FracturedAsset) -> MaterialHint {
    asset
        .root_chunks
        .first()
        .map(|chunk_id| asset.chunk(*chunk_id).material_hint)
        .unwrap_or(MaterialHint::Generic)
}
