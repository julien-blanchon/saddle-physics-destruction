use bevy::prelude::*;

use crate::{BondAsset, ChunkAsset, DamageKind, FalloffCurve};

#[derive(Debug, Clone, Copy)]
pub struct DamageProfile {
    pub origin: Vec3,
    pub direction: Vec3,
    pub magnitude: f32,
    pub radius: f32,
    pub kind: DamageKind,
    pub falloff: FalloffCurve,
}

impl DamageProfile {
    fn radius_or_unit(self) -> f32 {
        self.radius.max(0.001)
    }

    fn direction_or_default(self) -> Vec3 {
        self.direction.try_normalize().unwrap_or(Vec3::Y)
    }
}

pub fn chunk_damage(profile: DamageProfile, chunk: &ChunkAsset) -> f32 {
    let offset = chunk.centroid - profile.origin;
    let distance = offset.length() / profile.radius_or_unit();
    let falloff = profile.falloff.sample(distance);
    let directional = directional_term(profile, offset);

    let kind_scale = match profile.kind {
        DamageKind::Point => 1.1,
        DamageKind::Radial => 1.0,
        DamageKind::Directional => 0.8 + 0.6 * directional,
        DamageKind::Shear => 0.7 + 0.5 * (1.0 - directional.abs()),
    };

    profile.magnitude * falloff * kind_scale * chunk.damage_preview_weight.max(0.1)
}

pub fn bond_damage(profile: DamageProfile, bond: &BondAsset) -> f32 {
    let offset = bond.center - profile.origin;
    let distance = offset.length() / profile.radius_or_unit();
    let falloff = profile.falloff.sample(distance);
    let axis_alignment = directional_term(profile, offset);
    let shear_alignment = 1.0
        - bond
            .normal
            .try_normalize()
            .unwrap_or(Vec3::Y)
            .dot(profile.direction_or_default())
            .abs();

    let kind_scale = match profile.kind {
        DamageKind::Point => 1.0,
        DamageKind::Radial => 0.9,
        DamageKind::Directional => 0.65 + 0.75 * axis_alignment,
        DamageKind::Shear => 0.55 + 0.9 * shear_alignment,
    };

    profile.magnitude * falloff * kind_scale * bond.material_weight.max(0.1)
}

pub fn normalized_damage(total_damage: f32, threshold: f32) -> f32 {
    if threshold <= f32::EPSILON {
        1.0
    } else {
        (total_damage / threshold).clamp(0.0, 1.0)
    }
}

fn directional_term(profile: DamageProfile, offset: Vec3) -> f32 {
    let direction = profile.direction_or_default();
    offset
        .try_normalize()
        .unwrap_or(direction)
        .dot(direction)
        .max(0.0)
}

#[cfg(test)]
#[path = "damage_tests.rs"]
mod tests;
