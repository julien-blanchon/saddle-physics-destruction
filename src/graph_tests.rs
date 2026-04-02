use super::*;
use crate::components::DestructionRuntime;
use crate::{CuboidAnchorPreset, MaterialHint, authoring::CuboidFractureBuilder};

#[test]
fn unsupported_chunks_form_disjoint_groups() {
    let mut builder = CuboidFractureBuilder::new(Vec3::splat(3.0), UVec3::new(2, 2, 1));
    builder.anchor_preset = CuboidAnchorPreset::Bottom;
    builder.material_hint = MaterialHint::Stone;
    let asset = builder.build();
    let mut runtime = DestructionRuntime::from_asset(&asset);

    for bond in &asset.bonds {
        let y = asset.chunk(bond.chunks[0]).centroid.y + asset.chunk(bond.chunks[1]).centroid.y;
        if y > 0.0 {
            runtime.bond_health[bond.id.index()] = 0.0;
        }
    }

    let islands = unsupported_islands(&asset, &runtime, &asset.fixed_support_chunks());
    assert_eq!(islands.len(), 2);
    assert_eq!(islands[0].len() + islands[1].len(), 2);
}

#[test]
fn free_floating_assets_keep_one_primary_component() {
    let mut builder = CuboidFractureBuilder::new(Vec3::splat(2.0), UVec3::new(2, 1, 1));
    builder.anchor_preset = CuboidAnchorPreset::None;
    let asset = builder.build();
    let mut runtime = DestructionRuntime::from_asset(&asset);

    runtime.bond_health[0] = 0.0;

    let islands = unsupported_islands(&asset, &runtime, &[]);
    assert_eq!(islands.len(), 1);
    assert_eq!(islands[0].len(), 1);
}
