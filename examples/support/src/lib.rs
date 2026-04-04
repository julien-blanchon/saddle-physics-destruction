use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
use bevy::prelude::*;
use bevy_flair::FlairPlugin;
use bevy_input_focus::{InputDispatchPlugin, tab_navigation::TabNavigationPlugin};
use bevy_ui_widgets::UiWidgetsPlugins;
use saddle_pane::prelude::*;
use saddle_physics_destruction::{
    ApplyDestructionDamage, Destructible, DestructionAssetHandle, DestructionConfig,
    DestructionAvianFragments, DestructionDebugConfig, DestructionDiagnostics,
    DestructionEffectHooks, DestructionEffectStage, DestructionEffectTriggered,
    DestructionPlugin, DestructionState, DestructionViewers, FractureBias, FracturedAsset,
    Fragment, FragmentLifetime, FragmentRenderData, FragmentSpawnData, MaterialHint,
    RootVisualMode, build_fragment_mesh,
};

#[derive(Component)]
pub struct DiagnosticsText;

#[derive(Component)]
struct BreakPulse {
    timer: Timer,
    base_radius: f32,
    max_radius: f32,
}

#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
struct EffectHookDiagnostics {
    total_effect_triggers: u64,
    last_stage: String,
    last_audio_cue: String,
    last_particle_cue: String,
}

#[derive(Resource, Pane)]
#[pane(title = "Destruction Controls", position = "top-right")]
struct DestructionPane {
    #[pane(slider, min = 16.0, max = 256.0, step = 1.0)]
    fragment_budget: usize,
    #[pane(slider, min = 1.0, max = 12.0, step = 0.1)]
    default_fragment_lifetime_secs: f32,
    #[pane(slider, min = 0.0, max = 2.0, step = 0.05)]
    fragment_fade_secs: f32,
    #[pane(slider, min = 10.0, max = 120.0, step = 1.0)]
    max_fragment_distance: f32,
    #[pane(slider, min = 1.0, max = 64.0, step = 1.0)]
    max_chunk_spawns_per_frame: usize,
    draw_support_graph: bool,
}

impl FromWorld for DestructionPane {
    fn from_world(world: &mut World) -> Self {
        let config = world.resource::<DestructionConfig>().clone();
        let debug = world.resource::<DestructionDebugConfig>().clone();
        Self {
            fragment_budget: config.fragment_budget,
            default_fragment_lifetime_secs: config.default_fragment_lifetime_secs,
            fragment_fade_secs: config.fragment_fade_secs,
            max_fragment_distance: config.max_fragment_distance,
            max_chunk_spawns_per_frame: config.max_chunk_spawns_per_frame,
            draw_support_graph: debug.draw_support_graph,
        }
    }
}

#[derive(Resource, Default, Pane)]
#[pane(title = "Destruction Stats", position = "bottom-right")]
struct DestructionStatsPane {
    #[pane(monitor)]
    active_fragments: usize,
    #[pane(monitor)]
    pending_groups: usize,
    #[pane(monitor)]
    total_detached_groups: u64,
    #[pane(monitor)]
    total_support_evaluations: u64,
    #[pane(monitor)]
    total_effect_triggers: u64,
    #[pane(monitor)]
    last_effect_stage: String,
    #[pane(monitor)]
    last_audio_cue: String,
    #[pane(monitor)]
    last_particle_cue: String,
}

pub fn add_base_plugins(app: &mut App) {
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "destruction example".into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins((
        FlairPlugin,
        InputDispatchPlugin,
        UiWidgetsPlugins,
        TabNavigationPlugin,
        PanePlugin,
    ))
    .add_plugins(PhysicsPlugins::default())
    .add_plugins(DestructionPlugin::default())
    .init_resource::<EffectHookDiagnostics>()
    .register_pane::<DestructionPane>()
    .register_pane::<DestructionStatsPane>()
    .add_systems(Startup, setup_world)
    .add_systems(
        Update,
        (
            sync_pane_to_config,
            sync_stats_pane,
            sync_viewers_from_camera,
            materialize_fragments,
            record_effect_hooks,
            animate_break_pulses,
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
            DestructionAvianFragments::default(),
            DestructionEffectHooks {
                start_audio_cue: Some("destruction.start".into()),
                start_particle_cue: Some("dust_ring".into()),
                detach_audio_cue: Some("destruction.detach".into()),
                detach_particle_cue: Some("debris_burst".into()),
                final_audio_cue: Some("destruction.final".into()),
                final_particle_cue: Some("collapse_flash".into()),
            },
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
        RigidBody::Static,
        Collider::cuboid(40.0, 0.2, 40.0),
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
        ));
    }
}

fn record_effect_hooks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut diagnostics: ResMut<EffectHookDiagnostics>,
    mut reader: MessageReader<DestructionEffectTriggered>,
) {
    for message in reader.read() {
        diagnostics.total_effect_triggers += 1;
        diagnostics.last_stage = effect_stage_label(message.stage).to_string();
        diagnostics.last_audio_cue = message.audio_cue.clone().unwrap_or_else(|| "-".into());
        diagnostics.last_particle_cue = message
            .particle_cue
            .clone()
            .unwrap_or_else(|| "-".into());

        let flash_color = effect_color(message.stage, message.material_hint);
        let radius = 0.18 + message.fragment_count.max(1) as f32 * 0.04;
        commands.spawn((
            Name::new("Break Pulse"),
            BreakPulse {
                timer: Timer::from_seconds(0.45, TimerMode::Once),
                base_radius: radius,
                max_radius: radius + 0.65,
            },
            Mesh3d(meshes.add(Sphere::new(radius).mesh().ico(4).expect("pulse sphere"))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: flash_color.with_alpha(0.65),
                emissive: flash_color.to_linear(),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(message.world_position),
        ));
    }
}

fn animate_break_pulses(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pulses: Query<
        (
            Entity,
            &mut BreakPulse,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        ),
    >,
) {
    for (entity, mut pulse, mut transform, material_handle) in &mut pulses {
        if pulse.timer.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = pulse.timer.fraction();
        let radius = pulse.base_radius + (pulse.max_radius - pulse.base_radius) * progress;
        transform.scale = Vec3::splat(radius / pulse.base_radius.max(0.01));

        if let Some(material) = materials.get_mut(&material_handle.0) {
            let alpha = (1.0 - progress).powi(2) * 0.65;
            material.base_color = material.base_color.with_alpha(alpha);
            material.emissive = material.base_color.to_linear();
        }
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
    effect_hooks: Res<EffectHookDiagnostics>,
    destructibles: Query<(&Name, &DestructionState), With<Destructible>>,
    mut text: Single<&mut Text, With<DiagnosticsText>>,
) {
    let mut lines = vec![
        format!("fragments: {}", diagnostics.active_fragments),
        format!("pending groups: {}", diagnostics.pending_groups),
        format!("support evals: {}", diagnostics.total_support_evaluations),
        format!(
            "last cue: {} / {} / {}",
            effect_hooks.last_stage,
            effect_hooks.last_audio_cue,
            effect_hooks.last_particle_cue
        ),
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

fn sync_pane_to_config(
    pane: Res<DestructionPane>,
    mut config: ResMut<DestructionConfig>,
    mut debug: ResMut<DestructionDebugConfig>,
) {
    if !pane.is_changed() && !pane.is_added() {
        return;
    }

    config.fragment_budget = pane.fragment_budget;
    config.default_fragment_lifetime_secs = pane.default_fragment_lifetime_secs;
    config.fragment_fade_secs = pane.fragment_fade_secs;
    config.max_fragment_distance = pane.max_fragment_distance;
    config.max_chunk_spawns_per_frame = pane.max_chunk_spawns_per_frame;
    debug.draw_support_graph = pane.draw_support_graph;
}

fn sync_stats_pane(
    diagnostics: Res<DestructionDiagnostics>,
    effect_hooks: Res<EffectHookDiagnostics>,
    mut pane: ResMut<DestructionStatsPane>,
) {
    pane.active_fragments = diagnostics.active_fragments;
    pane.pending_groups = diagnostics.pending_groups;
    pane.total_detached_groups = diagnostics.total_detached_groups;
    pane.total_support_evaluations = diagnostics.total_support_evaluations;
    pane.total_effect_triggers = effect_hooks.total_effect_triggers;
    pane.last_effect_stage = effect_hooks.last_stage.clone();
    pane.last_audio_cue = effect_hooks.last_audio_cue.clone();
    pane.last_particle_cue = effect_hooks.last_particle_cue.clone();
}

fn dominant_material(asset: &FracturedAsset) -> MaterialHint {
    asset
        .root_chunks
        .first()
        .map(|chunk_id| asset.chunk(*chunk_id).material_hint)
        .unwrap_or(MaterialHint::Generic)
}

fn effect_stage_label(stage: DestructionEffectStage) -> &'static str {
    match stage {
        DestructionEffectStage::Started => "started",
        DestructionEffectStage::Detached => "detached",
        DestructionEffectStage::FinalCollapse => "final",
    }
}

fn effect_color(stage: DestructionEffectStage, material_hint: MaterialHint) -> Color {
    match (stage, material_hint) {
        (DestructionEffectStage::Started, MaterialHint::Glass) => Color::srgb(0.72, 0.9, 1.0),
        (DestructionEffectStage::Started, MaterialHint::Wood) => Color::srgb(1.0, 0.78, 0.52),
        (DestructionEffectStage::Started, MaterialHint::Stone | MaterialHint::Concrete) => {
            Color::srgb(0.92, 0.92, 0.96)
        }
        (DestructionEffectStage::Started, MaterialHint::Metal) => Color::srgb(0.9, 0.94, 1.0),
        (DestructionEffectStage::Started, MaterialHint::Ceramic) => Color::srgb(0.99, 0.92, 0.86),
        (DestructionEffectStage::Started, MaterialHint::Generic) => Color::srgb(0.98, 0.8, 0.52),
        (DestructionEffectStage::Detached, MaterialHint::Glass) => Color::srgb(0.56, 0.82, 1.0),
        (DestructionEffectStage::Detached, MaterialHint::Wood) => Color::srgb(0.92, 0.66, 0.36),
        (DestructionEffectStage::Detached, MaterialHint::Stone | MaterialHint::Concrete) => {
            Color::srgb(0.84, 0.84, 0.88)
        }
        (DestructionEffectStage::Detached, MaterialHint::Metal) => Color::srgb(0.78, 0.86, 0.96),
        (DestructionEffectStage::Detached, MaterialHint::Ceramic) => Color::srgb(0.96, 0.88, 0.82),
        (DestructionEffectStage::Detached, MaterialHint::Generic) => Color::srgb(0.94, 0.7, 0.42),
        (DestructionEffectStage::FinalCollapse, MaterialHint::Glass) => Color::srgb(0.94, 0.62, 0.46),
        (DestructionEffectStage::FinalCollapse, MaterialHint::Wood) => Color::srgb(1.0, 0.5, 0.28),
        (DestructionEffectStage::FinalCollapse, MaterialHint::Stone | MaterialHint::Concrete) => {
            Color::srgb(1.0, 0.58, 0.34)
        }
        (DestructionEffectStage::FinalCollapse, MaterialHint::Metal) => {
            Color::srgb(1.0, 0.64, 0.38)
        }
        (DestructionEffectStage::FinalCollapse, MaterialHint::Ceramic) => {
            Color::srgb(1.0, 0.56, 0.36)
        }
        (DestructionEffectStage::FinalCollapse, MaterialHint::Generic) => {
            Color::srgb(1.0, 0.58, 0.32)
        }
    }
}
