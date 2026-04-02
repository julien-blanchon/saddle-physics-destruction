use super::*;
use crate::{ChunkAsset, ChunkId, ChunkTags, FragmentRenderData, MaterialHint, SupportKind};

fn sample_chunk() -> ChunkAsset {
    ChunkAsset {
        id: ChunkId::new(0),
        name: "Sample".into(),
        parent: None,
        children: Vec::new(),
        fracture_level: 0,
        support_node: true,
        local_transform: Transform::IDENTITY,
        centroid: Vec3::new(0.0, 0.0, 0.0),
        half_extents: Vec3::splat(0.5),
        damage_threshold: 1.0,
        damage_preview_weight: 1.0,
        mass_hint: 1.0,
        support: SupportKind::None,
        material_hint: MaterialHint::Generic,
        tags: ChunkTags::default(),
        render: FragmentRenderData::None,
        collider: None,
    }
}

#[test]
fn falloff_never_increases_with_distance() {
    let profile = DamageProfile {
        origin: Vec3::ZERO,
        direction: Vec3::X,
        magnitude: 1.0,
        radius: 10.0,
        kind: DamageKind::Radial,
        falloff: FalloffCurve::Linear,
    };
    let mut near = sample_chunk();
    let mut far = sample_chunk();
    near.centroid = Vec3::new(1.0, 0.0, 0.0);
    far.centroid = Vec3::new(6.0, 0.0, 0.0);

    assert!(chunk_damage(profile, &near) > chunk_damage(profile, &far));
}

#[test]
fn normalized_damage_clamps() {
    assert_eq!(normalized_damage(4.0, 2.0), 1.0);
    assert_eq!(normalized_damage(0.0, 2.0), 0.0);
}
