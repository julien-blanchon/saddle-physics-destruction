use saddle_physics_destruction_example_support as support;

use bevy::prelude::*;
use saddle_physics_destruction::{
    CuboidAnchorPreset, CuboidFractureBuilder, Destructible, FractureBias, MaterialHint,
    RootVisualMode, RuntimeFracture, ThinSurfaceAnchorPreset, ThinSurfaceFractureBuilder,
};
use support::{add_base_plugins, emit_damage, material_for_hint};

#[derive(Resource)]
struct RuntimeFractureDemo {
    targets: [Entity; 2],
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
) {
    commands.spawn((
        Name::new("Runtime Bench"),
        Mesh3d(meshes.add(Cuboid::new(6.0, 0.35, 2.4))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.2, 0.22),
            perceptual_roughness: 0.82,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.85, 0.0),
    ));

    let runtime_crate = commands
        .spawn((
            Name::new("Runtime Cargo Crate"),
            Destructible {
                visual_mode: RootVisualMode::HideOnFirstDetach,
            },
            RuntimeFracture::cuboid({
                let mut builder =
                    CuboidFractureBuilder::new(Vec3::new(1.8, 1.8, 1.8), UVec3::new(3, 2, 2));
                builder.coarse_groups = Some(UVec3::new(3, 1, 1));
                builder.anchor_preset = CuboidAnchorPreset::None;
                builder.material_hint = MaterialHint::Wood;
                builder.seed = 18;
                builder
            }),
            Mesh3d(meshes.add(Cuboid::new(1.8, 1.8, 1.8))),
            MeshMaterial3d(materials.add(material_for_hint(MaterialHint::Wood, 1.0))),
            Transform::from_xyz(-1.8, 1.95, 0.0),
        ))
        .id();

    let runtime_glass = commands
        .spawn((
            Name::new("Runtime Glass Panel"),
            Destructible {
                visual_mode: RootVisualMode::HideOnFirstDetach,
            },
            RuntimeFracture::thin_surface({
                let mut builder = ThinSurfaceFractureBuilder::new(Vec2::new(2.4, 2.0), 0.08, 14);
                builder.anchor_preset = ThinSurfaceAnchorPreset::Frame;
                builder.material_hint = MaterialHint::Glass;
                builder.seed = 31;
                builder
            }),
            Mesh3d(meshes.add(Cuboid::new(2.4, 2.0, 0.08))),
            MeshMaterial3d(materials.add(material_for_hint(MaterialHint::Glass, 1.0))),
            Transform::from_xyz(2.1, 2.05, 0.0),
        ))
        .id();

    commands.insert_resource(RuntimeFractureDemo {
        targets: [runtime_crate, runtime_glass],
        timer: Timer::from_seconds(1.15, TimerMode::Repeating),
        pulse: 0,
    });
}

fn cycle_damage(
    time: Res<Time>,
    mut demo: ResMut<RuntimeFractureDemo>,
    mut writer: MessageWriter<saddle_physics_destruction::ApplyDestructionDamage>,
) {
    if !demo.timer.tick(time.delta()).just_finished() {
        return;
    }

    let target = demo.targets[demo.pulse % demo.targets.len()];
    let (origin, direction, magnitude, radius, bias) = if demo.pulse.is_multiple_of(2) {
        (
            Vec3::new(-1.8, 2.0, 0.7),
            Vec3::new(0.2, 0.1, -1.0),
            2.6,
            1.2,
            FractureBias::Balanced,
        )
    } else {
        (
            Vec3::new(2.1, 2.0, 0.3),
            Vec3::new(0.0, 0.0, -1.0),
            1.8,
            1.0,
            FractureBias::Fine,
        )
    };

    emit_damage(&mut writer, target, origin, direction, magnitude, radius, bias);
    demo.pulse += 1;
}
