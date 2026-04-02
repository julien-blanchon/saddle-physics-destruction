use saddle_physics_destruction_example_support as support;

use bevy::prelude::*;
use saddle_physics_saddle_physics_destruction::{CuboidAnchorPreset, CuboidFractureBuilder, FractureBias, MaterialHint};
use support::{add_base_plugins, emit_damage, spawn_preview_root};

#[derive(Resource)]
struct StressTargets {
    entities: Vec<Entity>,
    timer: Timer,
    cursor: usize,
}

fn main() {
    let mut app = App::new();
    add_base_plugins(&mut app);
    app.insert_resource(saddle_physics_destruction::DestructionConfig {
        fragment_budget: 96,
        default_fragment_lifetime_secs: 4.0,
        fragment_fade_secs: 0.6,
        max_chunk_spawns_per_frame: 40,
        ..default()
    })
    .add_systems(Startup, setup_example)
    .add_systems(Update, spam_impacts);
    app.run();
}

fn setup_example(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<Assets<saddle_physics_destruction::FracturedAsset>>,
) {
    let mut entities = Vec::new();
    for z in 0..4 {
        for x in 0..4 {
            let mut builder =
                CuboidFractureBuilder::new(Vec3::new(1.4, 1.4, 1.4), UVec3::new(2, 2, 2));
            builder.coarse_groups = Some(UVec3::new(2, 1, 1));
            builder.anchor_preset = CuboidAnchorPreset::None;
            builder.material_hint = MaterialHint::Concrete;
            builder.seed = 200 + (z * 4 + x) as u64;
            let asset = builder.build();
            let handle = assets.add(asset.clone());
            entities.push(spawn_preview_root(
                &mut commands,
                &mut meshes,
                &mut materials,
                handle,
                &asset,
                &format!("Stress Block {x}-{z}"),
                Transform::from_xyz(x as f32 * 2.3 - 3.4, 0.8, z as f32 * 2.2 - 3.2),
                saddle_physics_destruction::RootVisualMode::HideOnFirstDetach,
            ));
        }
    }

    commands.insert_resource(StressTargets {
        entities,
        timer: Timer::from_seconds(0.18, TimerMode::Repeating),
        cursor: 0,
    });
}

fn spam_impacts(
    time: Res<Time>,
    mut targets: ResMut<StressTargets>,
    mut writer: MessageWriter<saddle_physics_destruction::ApplyDestructionDamage>,
) {
    if !targets.timer.tick(time.delta()).just_finished() {
        return;
    }

    let entity = targets.entities[targets.cursor % targets.entities.len()];
    let x = ((targets.cursor % 4) as f32 - 1.5) * 2.3;
    let z = ((targets.cursor / 4 % 4) as f32 - 1.5) * 2.2;
    emit_damage(
        &mut writer,
        entity,
        Vec3::new(x, 0.9, z),
        Vec3::new(0.2, 0.0, -1.0),
        2.2,
        0.9,
        FractureBias::Balanced,
    );
    targets.cursor += 1;
}
