use bevy::prelude::*;

use crate::{
    BondAsset, BondId, ChunkAsset, ChunkId, ChunkTags, ColliderSource, FractureGenerator,
    FractureMetadata, FracturedAsset, FragmentRenderData, MaterialHint, SupportKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum CuboidAnchorPreset {
    #[default]
    Bottom,
    Top,
    None,
}

#[derive(Debug, Clone, Reflect)]
pub struct CuboidFractureBuilder {
    pub size: Vec3,
    pub cells: UVec3,
    pub coarse_groups: Option<UVec3>,
    pub seed: u64,
    pub jitter: f32,
    pub material_hint: MaterialHint,
    pub anchor_preset: CuboidAnchorPreset,
}

impl CuboidFractureBuilder {
    pub fn new(size: Vec3, cells: UVec3) -> Self {
        Self {
            size,
            cells,
            coarse_groups: None,
            seed: 1,
            jitter: 0.2,
            material_hint: MaterialHint::Concrete,
            anchor_preset: CuboidAnchorPreset::Bottom,
        }
    }

    pub fn build(&self) -> FracturedAsset {
        let mut rng = Lcg64::new(self.seed);
        let x_splits = split_axis(self.size.x, self.cells.x as usize, self.jitter, &mut rng);
        let y_splits = split_axis(self.size.y, self.cells.y as usize, self.jitter, &mut rng);
        let z_splits = split_axis(self.size.z, self.cells.z as usize, self.jitter, &mut rng);

        let mut chunks = Vec::new();
        let mut root_chunks = Vec::new();
        let mut support_chunks = Vec::new();
        let mut leaf_indices =
            vec![ChunkId::new(0); (self.cells.x * self.cells.y * self.cells.z) as usize];

        for z in 0..self.cells.z {
            for y in 0..self.cells.y {
                for x in 0..self.cells.x {
                    let min = Vec3::new(
                        x_splits[x as usize],
                        y_splits[y as usize],
                        z_splits[z as usize],
                    );
                    let max = Vec3::new(
                        x_splits[x as usize + 1],
                        y_splits[y as usize + 1],
                        z_splits[z as usize + 1],
                    );
                    let center = (min + max) * 0.5;
                    let size = max - min;
                    let id = ChunkId::new(chunks.len() as u32);
                    let support = match self.anchor_preset {
                        CuboidAnchorPreset::Bottom if y == 0 => SupportKind::Fixed,
                        CuboidAnchorPreset::Top if y == self.cells.y.saturating_sub(1) => {
                            SupportKind::Hanging
                        }
                        CuboidAnchorPreset::None => SupportKind::None,
                        _ => SupportKind::None,
                    };

                    chunks.push(ChunkAsset {
                        id,
                        name: format!("Leaf {x}-{y}-{z}"),
                        parent: None,
                        children: Vec::new(),
                        fracture_level: 1,
                        support_node: true,
                        local_transform: Transform::from_translation(center),
                        centroid: center,
                        half_extents: size * 0.5,
                        damage_threshold: 1.25 + size.length() * 0.35,
                        damage_preview_weight: 1.0,
                        mass_hint: size.x * size.y * size.z,
                        support,
                        material_hint: self.material_hint,
                        tags: ChunkTags {
                            load_bearing: support.is_anchor(),
                            ..default()
                        },
                        render: FragmentRenderData::Cuboid {
                            size,
                            interior_material_slot: 1,
                        },
                        collider: Some(ColliderSource::Cuboid { size }),
                    });
                    support_chunks.push(id);
                    leaf_indices[leaf_linear_index(self.cells, UVec3::new(x, y, z))] = id;
                }
            }
        }

        if let Some(group_cells) = self.coarse_groups {
            let group_size = UVec3::new(
                (self.cells.x / group_cells.x.max(1)).max(1),
                (self.cells.y / group_cells.y.max(1)).max(1),
                (self.cells.z / group_cells.z.max(1)).max(1),
            );

            for gz in 0..group_cells.z {
                for gy in 0..group_cells.y {
                    for gx in 0..group_cells.x {
                        let start =
                            UVec3::new(gx * group_size.x, gy * group_size.y, gz * group_size.z);
                        let end = UVec3::new(
                            ((gx + 1) * group_size.x).min(self.cells.x),
                            ((gy + 1) * group_size.y).min(self.cells.y),
                            ((gz + 1) * group_size.z).min(self.cells.z),
                        );

                        let mut children = Vec::new();
                        let mut min = Vec3::splat(f32::MAX);
                        let mut max = Vec3::splat(f32::MIN);

                        for z in start.z..end.z {
                            for y in start.y..end.y {
                                for x in start.x..end.x {
                                    let child = leaf_indices
                                        [leaf_linear_index(self.cells, UVec3::new(x, y, z))];
                                    children.push(child);
                                    let leaf = &chunks[child.index()];
                                    min = min.min(leaf.centroid - leaf.half_extents);
                                    max = max.max(leaf.centroid + leaf.half_extents);
                                }
                            }
                        }

                        let center = (min + max) * 0.5;
                        let size = max - min;
                        let parent_id = ChunkId::new(chunks.len() as u32);
                        for child in &children {
                            chunks[child.index()].parent = Some(parent_id);
                        }
                        chunks.push(ChunkAsset {
                            id: parent_id,
                            name: format!("Cluster {gx}-{gy}-{gz}"),
                            parent: None,
                            children,
                            fracture_level: 0,
                            support_node: false,
                            local_transform: Transform::from_translation(center),
                            centroid: center,
                            half_extents: size * 0.5,
                            damage_threshold: 3.5 + size.length() * 0.25,
                            damage_preview_weight: 0.35,
                            mass_hint: size.x * size.y * size.z,
                            support: SupportKind::None,
                            material_hint: self.material_hint,
                            tags: ChunkTags::default(),
                            render: FragmentRenderData::Cuboid {
                                size,
                                interior_material_slot: 1,
                            },
                            collider: Some(ColliderSource::Cuboid { size }),
                        });
                        root_chunks.push(parent_id);
                    }
                }
            }
        } else {
            root_chunks.extend(chunks.iter().map(|chunk| chunk.id));
        }

        let mut bonds = Vec::new();
        for z in 0..self.cells.z {
            for y in 0..self.cells.y {
                for x in 0..self.cells.x {
                    let current = leaf_indices[leaf_linear_index(self.cells, UVec3::new(x, y, z))];
                    if x + 1 < self.cells.x {
                        bonds.push(make_cuboid_bond(
                            &chunks,
                            current,
                            leaf_indices[leaf_linear_index(self.cells, UVec3::new(x + 1, y, z))],
                            Vec3::X,
                        ));
                    }
                    if y + 1 < self.cells.y {
                        bonds.push(make_cuboid_bond(
                            &chunks,
                            current,
                            leaf_indices[leaf_linear_index(self.cells, UVec3::new(x, y + 1, z))],
                            Vec3::Y,
                        ));
                    }
                    if z + 1 < self.cells.z {
                        bonds.push(make_cuboid_bond(
                            &chunks,
                            current,
                            leaf_indices[leaf_linear_index(self.cells, UVec3::new(x, y, z + 1))],
                            Vec3::Z,
                        ));
                    }
                }
            }
        }

        for (index, bond) in bonds.iter_mut().enumerate() {
            bond.id = BondId::new(index as u32);
        }

        FracturedAsset {
            metadata: FractureMetadata {
                seed: self.seed,
                generator: FractureGenerator::JitteredGrid,
                notes: "deterministic cuboid partition".into(),
            },
            bounds: self.size,
            chunks,
            bonds,
            root_chunks,
            support_chunks,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum ThinSurfaceAnchorPreset {
    #[default]
    BottomEdge,
    Frame,
    None,
}

#[derive(Debug, Clone, Reflect)]
pub struct ThinSurfaceFractureBuilder {
    pub size: Vec2,
    pub depth: f32,
    pub cells: usize,
    pub seed: u64,
    pub anchor_preset: ThinSurfaceAnchorPreset,
    pub material_hint: MaterialHint,
}

impl ThinSurfaceFractureBuilder {
    pub fn new(size: Vec2, depth: f32, cells: usize) -> Self {
        Self {
            size,
            depth,
            cells,
            seed: 7,
            anchor_preset: ThinSurfaceAnchorPreset::BottomEdge,
            material_hint: MaterialHint::Glass,
        }
    }

    pub fn build(&self) -> FracturedAsset {
        let mut rng = Lcg64::new(self.seed);
        let sites = generate_sites(self.size, self.cells, &mut rng);
        let mut chunks = Vec::new();
        let mut support_chunks = Vec::new();

        for (site_index, site) in sites.iter().enumerate() {
            let polygon = build_voronoi_cell(*site, &sites, self.size);
            if polygon.len() < 3 {
                continue;
            }
            let centroid = polygon_centroid(&polygon);
            let local_polygon: Vec<Vec2> = polygon.iter().map(|point| *point - centroid).collect();
            let bounds = local_polygon
                .iter()
                .fold(Vec2::ZERO, |acc, point| acc.max(point.abs()));

            let id = ChunkId::new(chunks.len() as u32);
            let support = match self.anchor_preset {
                ThinSurfaceAnchorPreset::BottomEdge if centroid.y <= -self.size.y * 0.5 + 0.18 => {
                    SupportKind::Fixed
                }
                ThinSurfaceAnchorPreset::Frame
                    if centroid.x.abs() >= self.size.x * 0.42
                        || centroid.y.abs() >= self.size.y * 0.42 =>
                {
                    SupportKind::Fixed
                }
                ThinSurfaceAnchorPreset::None => SupportKind::None,
                _ => SupportKind::None,
            };

            let hull = local_polygon
                .iter()
                .flat_map(|point| {
                    [
                        Vec3::new(point.x, point.y, self.depth * 0.5),
                        Vec3::new(point.x, point.y, -self.depth * 0.5),
                    ]
                })
                .collect();

            chunks.push(ChunkAsset {
                id,
                name: format!("Panel Cell {site_index}"),
                parent: None,
                children: Vec::new(),
                fracture_level: 1,
                support_node: true,
                local_transform: Transform::from_translation(Vec3::new(
                    centroid.x, centroid.y, 0.0,
                )),
                centroid: Vec3::new(centroid.x, centroid.y, 0.0),
                half_extents: Vec3::new(bounds.x.max(0.02), bounds.y.max(0.02), self.depth * 0.5),
                damage_threshold: 0.7 + bounds.length() * 0.25,
                damage_preview_weight: 1.1,
                mass_hint: polygon_area(&local_polygon).max(0.02) * self.depth.max(0.02),
                support,
                material_hint: self.material_hint,
                tags: ChunkTags {
                    load_bearing: support.is_anchor(),
                    ..default()
                },
                render: FragmentRenderData::ExtrudedConvex {
                    polygon: local_polygon.clone(),
                    depth: self.depth,
                    interior_material_slot: 1,
                },
                collider: Some(ColliderSource::ConvexHull(hull)),
            });
            support_chunks.push(id);
        }

        let mut bonds = Vec::new();
        for left in 0..chunks.len() {
            for right in left + 1..chunks.len() {
                let left_chunk = &chunks[left];
                let right_chunk = &chunks[right];
                let distance = left_chunk.centroid.distance(right_chunk.centroid);
                let reach = left_chunk.half_extents.truncate().length()
                    + right_chunk.half_extents.truncate().length();
                if distance > reach * 0.78 {
                    continue;
                }

                let delta = (right_chunk.centroid - left_chunk.centroid).normalize_or_zero();
                bonds.push(BondAsset {
                    id: BondId::new(bonds.len() as u32),
                    chunks: [left_chunk.id, right_chunk.id],
                    health: 0.7 + (reach - distance).max(0.05) * 0.4,
                    material_weight: 0.8,
                    center: (left_chunk.centroid + right_chunk.centroid) * 0.5,
                    normal: delta,
                    fracture_level: 1,
                });
            }
        }

        let root = ChunkId::new(chunks.len() as u32);
        for chunk in &mut chunks {
            chunk.parent = Some(root);
        }

        let mut all_chunks = chunks;
        all_chunks.push(ChunkAsset {
            id: root,
            name: "Panel Root".into(),
            parent: None,
            children: support_chunks.clone(),
            fracture_level: 0,
            support_node: false,
            local_transform: Transform::IDENTITY,
            centroid: Vec3::ZERO,
            half_extents: Vec3::new(self.size.x * 0.5, self.size.y * 0.5, self.depth * 0.5),
            damage_threshold: 4.0,
            damage_preview_weight: 0.25,
            mass_hint: self.size.x * self.size.y * self.depth,
            support: SupportKind::None,
            material_hint: self.material_hint,
            tags: ChunkTags::default(),
            render: FragmentRenderData::ExtrudedConvex {
                polygon: vec![
                    Vec2::new(-self.size.x * 0.5, -self.size.y * 0.5),
                    Vec2::new(self.size.x * 0.5, -self.size.y * 0.5),
                    Vec2::new(self.size.x * 0.5, self.size.y * 0.5),
                    Vec2::new(-self.size.x * 0.5, self.size.y * 0.5),
                ],
                depth: self.depth,
                interior_material_slot: 1,
            },
            collider: Some(ColliderSource::Cuboid {
                size: Vec3::new(self.size.x, self.size.y, self.depth),
            }),
        });

        FracturedAsset {
            metadata: FractureMetadata {
                seed: self.seed,
                generator: FractureGenerator::ThinSurfaceVoronoi,
                notes: "deterministic 2D Voronoi cell fracture".into(),
            },
            bounds: Vec3::new(self.size.x, self.size.y, self.depth),
            chunks: all_chunks,
            bonds,
            root_chunks: vec![root],
            support_chunks,
        }
    }
}

fn split_axis(total_size: f32, cells: usize, jitter: f32, rng: &mut Lcg64) -> Vec<f32> {
    let mut positions = Vec::with_capacity(cells + 1);
    positions.push(-total_size * 0.5);
    let step = total_size / cells.max(1) as f32;

    for index in 1..cells {
        let ideal = -total_size * 0.5 + step * index as f32;
        let jitter_amount = step * jitter.clamp(0.0, 0.49) * rng.next_signed_f32();
        positions.push(ideal + jitter_amount);
    }

    positions.push(total_size * 0.5);
    positions.sort_by(f32::total_cmp);
    positions
}

fn make_cuboid_bond(
    chunks: &[ChunkAsset],
    left: ChunkId,
    right: ChunkId,
    normal: Vec3,
) -> BondAsset {
    let left_chunk = &chunks[left.index()];
    let right_chunk = &chunks[right.index()];
    let face_area = if normal == Vec3::X {
        left_chunk.half_extents.y * 2.0 * left_chunk.half_extents.z * 2.0
    } else if normal == Vec3::Y {
        left_chunk.half_extents.x * 2.0 * left_chunk.half_extents.z * 2.0
    } else {
        left_chunk.half_extents.x * 2.0 * left_chunk.half_extents.y * 2.0
    };

    BondAsset {
        id: BondId::new(0),
        chunks: [left, right],
        health: 0.9 + face_area * 0.25,
        material_weight: 1.0,
        center: (left_chunk.centroid + right_chunk.centroid) * 0.5,
        normal,
        fracture_level: 1,
    }
}

fn leaf_linear_index(cells: UVec3, coord: UVec3) -> usize {
    (coord.x + coord.y * cells.x + coord.z * cells.x * cells.y) as usize
}

fn generate_sites(size: Vec2, count: usize, rng: &mut Lcg64) -> Vec<Vec2> {
    (0..count)
        .map(|_| {
            Vec2::new(
                (rng.next_f32() - 0.5) * size.x,
                (rng.next_f32() - 0.5) * size.y,
            )
        })
        .collect()
}

fn build_voronoi_cell(site: Vec2, sites: &[Vec2], size: Vec2) -> Vec<Vec2> {
    let mut polygon = vec![
        Vec2::new(-size.x * 0.5, -size.y * 0.5),
        Vec2::new(size.x * 0.5, -size.y * 0.5),
        Vec2::new(size.x * 0.5, size.y * 0.5),
        Vec2::new(-size.x * 0.5, size.y * 0.5),
    ];

    for other in sites {
        if *other == site {
            continue;
        }
        let midpoint = (site + *other) * 0.5;
        let normal = (*other - site).normalize_or_zero();
        polygon = clip_polygon_with_half_plane(&polygon, midpoint, normal);
        if polygon.len() < 3 {
            break;
        }
    }

    polygon
}

fn clip_polygon_with_half_plane(polygon: &[Vec2], point_on_plane: Vec2, normal: Vec2) -> Vec<Vec2> {
    let mut output = Vec::new();
    if polygon.is_empty() {
        return output;
    }

    let mut previous = *polygon.last().unwrap_or(&Vec2::ZERO);
    let mut previous_inside = (previous - point_on_plane).dot(normal) <= 0.0;

    for current in polygon {
        let current_inside = (*current - point_on_plane).dot(normal) <= 0.0;
        if current_inside != previous_inside {
            let direction = *current - previous;
            let denominator = direction.dot(normal);
            if denominator.abs() > 0.000_01 {
                let t = (point_on_plane - previous).dot(normal) / denominator;
                output.push(previous + direction * t.clamp(0.0, 1.0));
            }
        }
        if current_inside {
            output.push(*current);
        }
        previous = *current;
        previous_inside = current_inside;
    }

    output
}

fn polygon_area(points: &[Vec2]) -> f32 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    0.5 * area.abs()
}

fn polygon_centroid(points: &[Vec2]) -> Vec2 {
    let mut cross_sum = 0.0;
    let mut centroid = Vec2::ZERO;

    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        let cross = points[index].x * points[next].y - points[next].x * points[index].y;
        cross_sum += cross;
        centroid += (points[index] + points[next]) * cross;
    }

    if cross_sum.abs() <= f32::EPSILON {
        return points.first().copied().unwrap_or(Vec2::ZERO);
    }

    centroid / (3.0 * cross_sum)
}

#[derive(Debug, Clone)]
struct Lcg64 {
    state: u64,
}

impl Lcg64 {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn next_signed_f32(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

#[cfg(test)]
#[path = "authoring_tests.rs"]
mod tests;
