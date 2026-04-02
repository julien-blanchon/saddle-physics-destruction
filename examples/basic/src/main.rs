use saddle_physics_destruction_example_support as support;

use bevy::prelude::*;
use saddle_physics_destruction::{
    CuboidAnchorPreset, CuboidFractureBuilder, FractureBias, MaterialHint,
};
use support::{add_base_plugins, emit_damage, spawn_preview_root};

#[derive(Resource)]
struct DemoTarget {
    entity: Entity,
    timer: Timer,
    pulse: usize,
}

fn main() {
    let mut app = App::new();
    add_base_plugins(&mut app);
    app.add_systems(Startup, setup_example)
        .add_systems(Update, cycle_damage);
    app.run();
}

fn setup_example(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<Assets<saddle_physics_destruction::FracturedAsset>>,
) {
    let mut builder = CuboidFractureBuilder::new(Vec3::new(2.6, 2.0, 2.2), UVec3::new(3, 2, 2));
    builder.coarse_groups = Some(UVec3::new(3, 1, 1));
    builder.anchor_preset = CuboidAnchorPreset::None;
    builder.material_hint = MaterialHint::Wood;
    builder.seed = 12;
    let asset = builder.build();
    let handle = assets.add(asset.clone());
    let entity = spawn_preview_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        handle,
        &asset,
        "Breakable Crate",
        Transform::from_xyz(0.0, 1.05, 0.0),
        saddle_physics_destruction::RootVisualMode::HideOnFirstDetach,
    );

    commands.insert_resource(DemoTarget {
        entity,
        timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        pulse: 0,
    });
}

fn cycle_damage(
    time: Res<Time>,
    mut target: ResMut<DemoTarget>,
    mut writer: MessageWriter<saddle_physics_destruction::ApplyDestructionDamage>,
) {
    if !target.timer.tick(time.delta()).just_finished() {
        return;
    }

    let offsets = [
        Vec3::new(-0.9, 0.5, 0.6),
        Vec3::new(0.8, 0.7, -0.6),
        Vec3::new(0.0, 0.9, 0.0),
    ];
    let origin = offsets[target.pulse % offsets.len()];
    emit_damage(
        &mut writer,
        target.entity,
        origin,
        Vec3::new(0.0, 0.1, -1.0),
        2.4,
        1.4,
        FractureBias::Balanced,
    );
    target.pulse += 1;
}
