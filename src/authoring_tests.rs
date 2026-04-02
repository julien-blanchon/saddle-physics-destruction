use super::*;

#[test]
fn cuboid_builder_is_deterministic() {
    let mut builder = CuboidFractureBuilder::new(Vec3::new(4.0, 3.0, 2.0), UVec3::new(3, 2, 1));
    builder.coarse_groups = Some(UVec3::new(3, 1, 1));
    builder.seed = 42;

    let first = builder.build();
    let second = builder.build();

    assert_eq!(first.chunks.len(), second.chunks.len());
    assert_eq!(first.bonds.len(), second.bonds.len());
    assert_eq!(first.root_chunks, second.root_chunks);
    assert_eq!(first.support_chunks, second.support_chunks);
    assert_eq!(first.metadata.seed, second.metadata.seed);
    assert_eq!(first.metadata.generator, second.metadata.generator);
    assert!(first.validate().is_ok());
    assert!(
        first
            .chunks
            .iter()
            .zip(&second.chunks)
            .all(|(left, right)| left.centroid == right.centroid && left.parent == right.parent)
    );
    assert!(
        first
            .bonds
            .iter()
            .zip(&second.bonds)
            .all(|(left, right)| left.center == right.center && left.chunks == right.chunks)
    );
}

#[test]
fn thin_surface_builder_produces_stable_ids() {
    let builder = ThinSurfaceFractureBuilder::new(Vec2::new(3.0, 2.0), 0.06, 12);
    let asset = builder.build();

    for (index, chunk) in asset.chunks.iter().enumerate() {
        assert_eq!(chunk.id.index(), index);
    }

    assert!(asset.validate().is_ok());
}
