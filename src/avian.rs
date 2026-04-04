use avian3d::prelude::{
    AngularDamping, AngularVelocity, Collider, CollisionLayers, Friction, GravityScale,
    LinearDamping, LinearVelocity, Mass, Restitution, RigidBody,
};
use bevy::prelude::*;

use crate::{ColliderSource, Fragment, FragmentSpawnData};

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct DestructionAvianFragments {
    pub rigid_body: RigidBody,
    pub mass_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
    pub friction: f32,
    pub restitution: f32,
    pub collision_layers: Option<CollisionLayers>,
}

impl Default for DestructionAvianFragments {
    fn default() -> Self {
        Self {
            rigid_body: RigidBody::Dynamic,
            mass_scale: 1.0,
            linear_damping: 0.12,
            angular_damping: 0.2,
            gravity_scale: 1.0,
            friction: 0.8,
            restitution: 0.08,
            collision_layers: None,
        }
    }
}

pub(crate) fn attach_avian_fragment_bodies(
    mut commands: Commands,
    fragments: Query<(Entity, &Fragment, &FragmentSpawnData), Added<Fragment>>,
    roots: Query<&DestructionAvianFragments>,
) {
    for (entity, fragment, spawn_data) in &fragments {
        let Ok(config) = roots.get(fragment.source) else {
            continue;
        };
        let Some(collider) = collider_from_source(spawn_data.collider.as_ref()) else {
            continue;
        };

        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            config.rigid_body,
            collider,
            Mass((spawn_data.mass_hint * config.mass_scale).max(0.05)),
            LinearVelocity(spawn_data.initial_velocity.linear),
            AngularVelocity(spawn_data.initial_velocity.angular),
            LinearDamping(config.linear_damping),
            AngularDamping(config.angular_damping),
            GravityScale(config.gravity_scale),
            Friction::new(config.friction),
            Restitution::new(config.restitution),
        ));

        if let Some(layers) = config.collision_layers {
            entity_commands.insert(layers);
        }
    }
}

fn collider_from_source(source: Option<&ColliderSource>) -> Option<Collider> {
    match source? {
        ColliderSource::Cuboid { size } => Some(Collider::cuboid(size.x, size.y, size.z)),
        ColliderSource::ConvexHull(points) => Collider::convex_hull(points.clone()),
        ColliderSource::TriMesh { vertices, indices } => {
            let triangles = indices
                .chunks_exact(3)
                .map(|triangle| [triangle[0], triangle[1], triangle[2]])
                .collect::<Vec<_>>();
            if triangles.is_empty() {
                return None;
            }
            Some(Collider::trimesh(vertices.clone(), triangles))
        }
    }
}
