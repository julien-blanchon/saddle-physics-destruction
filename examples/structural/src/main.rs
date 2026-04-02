use saddle_physics_destruction_example_support as support;

use bevy::prelude::*;
use saddle_physics_saddle_physics_destruction::{
    CuboidAnchorPreset, CuboidFractureBuilder, FractureBias, MaterialHint,
};
use support::{add_base_plugins, emit_damage, spawn_preview_root};

#[derive(Resource)]
struct StructuralDemo {
    entity: Entity,
    timer: Timer,
    pulse: usize,
}

fn main() {
    let mut app = App::new();
    add_base_plugins(&mut app);
    app.add_systems(Startup, setup_example)
        .add_systems(Update, batter_base);
    app.run();
}

fn setup_example(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<Assets<saddle_physics_destruction::FracturedAsset>>,
) {
    let mut builder = CuboidFractureBuilder::new(Vec3::new(2.0, 5.4, 2.0), UVec3::new(2, 6, 1));
    builder.coarse_groups = Some(UVec3::new(2, 3, 1));
    builder.anchor_preset = CuboidAnchorPreset::Bottom;
    builder.material_hint = MaterialHint::Stone;
    builder.seed = 44;
    let asset = builder.build();
    let handle = assets.add(asset.clone());
    let entity = spawn_preview_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        handle,
        &asset,
        "Stone Pillar",
        Transform::from_xyz(0.0, 2.7, 0.0),
        saddle_physics_destruction::RootVisualMode::HideWhenBroken,
    );

    commands.insert_resource(StructuralDemo {
        entity,
        timer: Timer::from_seconds(1.1, TimerMode::Repeating),
        pulse: 0,
    });
}

fn batter_base(
    time: Res<Time>,
    mut demo: ResMut<StructuralDemo>,
    mut writer: MessageWriter<saddle_physics_destruction::ApplyDestructionDamage>,
) {
    if !demo.timer.tick(time.delta()).just_finished() {
        return;
    }

    let side = if demo.pulse.is_multiple_of(2) {
        -0.8
    } else {
        0.8
    };
    emit_damage(
        &mut writer,
        demo.entity,
        Vec3::new(side, 0.8, 0.0),
        Vec3::new(-side.signum(), 0.0, 0.0),
        3.0,
        1.2,
        FractureBias::Balanced,
    );
    demo.pulse += 1;
}
