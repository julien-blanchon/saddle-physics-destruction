use saddle_physics_destruction_example_support as support;

use bevy::prelude::*;
use saddle_physics_saddle_physics_destruction::{
    FractureBias, MaterialHint, ThinSurfaceAnchorPreset, ThinSurfaceFractureBuilder,
};
use support::{add_base_plugins, emit_damage, spawn_preview_root};

#[derive(Resource)]
struct GlassDemo {
    entity: Entity,
    timer: Timer,
}

fn main() {
    let mut app = App::new();
    add_base_plugins(&mut app);
    app.add_systems(Startup, setup_example)
        .add_systems(Update, shatter_panel);
    app.run();
}

fn setup_example(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<Assets<saddle_physics_destruction::FracturedAsset>>,
) {
    let mut builder = ThinSurfaceFractureBuilder::new(Vec2::new(4.0, 2.6), 0.05, 20);
    builder.anchor_preset = ThinSurfaceAnchorPreset::Frame;
    builder.material_hint = MaterialHint::Glass;
    builder.seed = 1024;
    let asset = builder.build();
    let handle = assets.add(asset.clone());
    let entity = spawn_preview_root(
        &mut commands,
        &mut meshes,
        &mut materials,
        handle,
        &asset,
        "Glass Panel",
        Transform::from_xyz(0.0, 1.6, 0.0),
        saddle_physics_destruction::RootVisualMode::HideOnFirstDetach,
    );

    commands.insert_resource(GlassDemo {
        entity,
        timer: Timer::from_seconds(0.9, TimerMode::Repeating),
    });
}

fn shatter_panel(
    time: Res<Time>,
    mut demo: ResMut<GlassDemo>,
    mut writer: MessageWriter<saddle_physics_destruction::ApplyDestructionDamage>,
) {
    if !demo.timer.tick(time.delta()).just_finished() {
        return;
    }

    emit_damage(
        &mut writer,
        demo.entity,
        Vec3::new(0.0, 1.6, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        2.3,
        0.95,
        FractureBias::Fine,
    );
}
