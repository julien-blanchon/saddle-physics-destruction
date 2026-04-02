use super::*;

#[test]
fn asset_validation_rejects_empty_assets() {
    let asset = FracturedAsset {
        metadata: FractureMetadata::default(),
        bounds: Vec3::ONE,
        chunks: Vec::new(),
        bonds: Vec::new(),
        root_chunks: Vec::new(),
        support_chunks: Vec::new(),
    };

    assert!(asset.validate().is_err());
}
