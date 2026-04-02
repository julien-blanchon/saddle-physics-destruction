use bevy::prelude::*;

use crate::{ChunkId, InitialVelocity, MaterialHint};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum DamageKind {
    #[default]
    Point,
    Radial,
    Directional,
    Shear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum FalloffCurve {
    Constant,
    #[default]
    Linear,
    SmoothStep,
    Quadratic,
}

impl FalloffCurve {
    pub fn sample(self, normalized_distance: f32) -> f32 {
        let t = normalized_distance.clamp(0.0, 1.0);
        match self {
            Self::Constant => 1.0,
            Self::Linear => 1.0 - t,
            Self::SmoothStep => {
                let inv = 1.0 - t;
                inv * inv * (3.0 - 2.0 * inv)
            }
            Self::Quadratic => (1.0 - t) * (1.0 - t),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum FractureBias {
    Coarse,
    #[default]
    Balanced,
    Fine,
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct ApplyDestructionDamage {
    pub target: Entity,
    pub origin: Vec3,
    pub direction: Vec3,
    pub magnitude: f32,
    pub radius: f32,
    pub kind: DamageKind,
    pub falloff: FalloffCurve,
    pub fracture_bias: FractureBias,
}

impl ApplyDestructionDamage {
    pub fn radial(target: Entity, origin: Vec3, magnitude: f32, radius: f32) -> Self {
        Self {
            target,
            origin,
            direction: Vec3::Y,
            magnitude,
            radius,
            kind: DamageKind::Radial,
            falloff: FalloffCurve::Linear,
            fracture_bias: FractureBias::Balanced,
        }
    }
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct DestructionStarted {
    pub source: Entity,
    pub world_position: Vec3,
    pub material_hint: MaterialHint,
    pub energy: f32,
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct ChunkGroupDetached {
    pub source: Entity,
    pub chunk_ids: Vec<ChunkId>,
    pub fragment_count: usize,
    pub world_position: Vec3,
    pub material_hint: MaterialHint,
    pub impulse: InitialVelocity,
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct FinalDestructionOccurred {
    pub source: Entity,
    pub world_position: Vec3,
    pub detached_groups: u32,
    pub chunk_count: usize,
    pub material_hint: MaterialHint,
}
