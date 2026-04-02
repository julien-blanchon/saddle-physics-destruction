use saddle_physics_destruction_example_support as support;

use bevy::prelude::*;
use saddle_physics_saddle_physics_destruction::{CuboidAnchorPreset, CuboidFractureBuilder, FractureBias, MaterialHint};
use support::{add_base_plugins, emit_damage, spawn_preview_root};

#[derive(Resource)]
struct HierarchyDemo {
    entity: Entity,
    timer: Timer,
    pulse: usize,
}

fn main() {
    let mut app = App::new();
    add_base_plugins(&mut app);
    app.add_systems(Startup, setup_example)
        .add_systems(Update, stage_damage);
    app.run();
}

fn setup_example(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<Assets<saddle_physics_destruction::FracturedAsset>>,
) {
    let mut builder = CuboidFractureBuilder::new(Vec3::new(3.0, 4.8, 2.6), UVec3::new(4, 4, 2));
    builder.coarse_groups = Some(UVec3::new(2, 4, 1));
    builder.anchor_preset = CuboidAnchorPreset::Bottom;
    builder.material_hint = MaterialHint::Concrete;
    builder.seed = 77;
    let asset = builder.build();
    let handle = assets.add(asset.clone());
    let entity = spawn_preview_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        handle,
        &asset,
        "Hero Prop",
        Transform::from_xyz(0.0, 2.5, 0.0),
        saddle_physics_destruction::RootVisualMode::HideOnFirstDetach,
    );

    commands.insert_resource(HierarchyDemo {
        entity,
        timer: Timer::from_seconds(1.05, TimerMode::Repeating),
        pulse: 0,
    });
}

fn stage_damage(
    time: Res<Time>,
    mut demo: ResMut<HierarchyDemo>,
    mut writer: MessageWriter<saddle_physics_destruction::ApplyDestructionDamage>,
) {
    if !demo.timer.tick(time.delta()).just_finished() {
        return;
    }

    let (origin, magnitude, bias) = match demo.pulse {
        0 | 1 => (Vec3::new(-0.9, 1.2, 0.5), 2.9, FractureBias::Coarse),
        2 | 3 => (Vec3::new(1.1, 2.0, -0.3), 3.2, FractureBias::Balanced),
        _ => (Vec3::new(0.0, 2.8, 0.0), 3.4, FractureBias::Fine),
    };

    emit_damage(
        &mut writer,
        demo.entity,
        origin,
        Vec3::new(0.0, 0.0, -1.0),
        magnitude,
        1.4,
        bias,
    );
    demo.pulse += 1;
}
