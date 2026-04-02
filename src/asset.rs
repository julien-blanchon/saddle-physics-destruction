use std::collections::VecDeque;

use bevy::{asset::Asset, prelude::*};

use crate::{BondId, ChunkId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum FractureGenerator {
    #[default]
    Manual,
    JitteredGrid,
    ThinSurfaceVoronoi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum MaterialHint {
    #[default]
    Generic,
    Glass,
    Wood,
    Stone,
    Concrete,
    Metal,
    Ceramic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum SupportKind {
    #[default]
    None,
    Fixed,
    Hanging,
    Weak,
}

impl SupportKind {
    pub const fn is_anchor(self) -> bool {
        matches!(self, Self::Fixed | Self::Hanging | Self::Weak)
    }
}

#[derive(Debug, Clone, PartialEq, Reflect, Default)]
pub struct ChunkTags {
    pub load_bearing: bool,
    pub cosmetic_only: bool,
    pub never_detach: bool,
    pub no_collider: bool,
    pub no_shadow: bool,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum ColliderSource {
    Cuboid {
        size: Vec3,
    },
    ConvexHull(Vec<Vec3>),
    TriMesh {
        vertices: Vec<Vec3>,
        indices: Vec<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum FragmentRenderData {
    Cuboid {
        size: Vec3,
        interior_material_slot: u8,
    },
    ExtrudedConvex {
        polygon: Vec<Vec2>,
        depth: f32,
        interior_material_slot: u8,
    },
    None,
}

#[derive(Debug, Clone, Reflect)]
pub struct FractureMetadata {
    pub seed: u64,
    pub generator: FractureGenerator,
    pub notes: String,
}

impl Default for FractureMetadata {
    fn default() -> Self {
        Self {
            seed: 0,
            generator: FractureGenerator::Manual,
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct ChunkAsset {
    pub id: ChunkId,
    pub name: String,
    pub parent: Option<ChunkId>,
    pub children: Vec<ChunkId>,
    pub fracture_level: u8,
    pub support_node: bool,
    pub local_transform: Transform,
    pub centroid: Vec3,
    pub half_extents: Vec3,
    pub damage_threshold: f32,
    pub damage_preview_weight: f32,
    pub mass_hint: f32,
    pub support: SupportKind,
    pub material_hint: MaterialHint,
    pub tags: ChunkTags,
    pub render: FragmentRenderData,
    pub collider: Option<ColliderSource>,
}

#[derive(Debug, Clone, Reflect)]
pub struct BondAsset {
    pub id: BondId,
    pub chunks: [ChunkId; 2],
    pub health: f32,
    pub material_weight: f32,
    pub center: Vec3,
    pub normal: Vec3,
    pub fracture_level: u8,
}

#[derive(Asset, Debug, Clone, Reflect)]
pub struct FracturedAsset {
    pub metadata: FractureMetadata,
    pub bounds: Vec3,
    pub chunks: Vec<ChunkAsset>,
    pub bonds: Vec<BondAsset>,
    pub root_chunks: Vec<ChunkId>,
    pub support_chunks: Vec<ChunkId>,
}

impl FracturedAsset {
    pub fn validate(&self) -> Result<(), String> {
        if self.chunks.is_empty() {
            return Err("fractured asset must contain at least one chunk".into());
        }

        for (expected, chunk) in self.chunks.iter().enumerate() {
            if chunk.id.index() != expected {
                return Err(format!(
                    "chunk ids must be contiguous, expected {}, got {}",
                    expected, chunk.id.0
                ));
            }

            if let Some(parent) = chunk.parent {
                self.chunks.get(parent.index()).ok_or_else(|| {
                    format!("chunk {} has invalid parent {}", chunk.id.0, parent.0)
                })?;
            }

            for child in &chunk.children {
                self.chunks.get(child.index()).ok_or_else(|| {
                    format!("chunk {} references invalid child {}", chunk.id.0, child.0)
                })?;
            }
        }

        for (expected, bond) in self.bonds.iter().enumerate() {
            if bond.id.index() != expected {
                return Err(format!(
                    "bond ids must be contiguous, expected {}, got {}",
                    expected, bond.id.0
                ));
            }
            for chunk in bond.chunks {
                self.chunks.get(chunk.index()).ok_or_else(|| {
                    format!("bond {} references invalid chunk {}", bond.id.0, chunk.0)
                })?;
            }
        }

        for &root in &self.root_chunks {
            let chunk = self.chunk(root);
            if chunk.parent.is_some() {
                return Err(format!(
                    "root chunk {} cannot also have a parent",
                    chunk.id.0
                ));
            }
        }

        for &support in &self.support_chunks {
            if !self.chunk(support).support_node {
                return Err(format!(
                    "support chunk list contains non-support chunk {}",
                    support.0
                ));
            }
        }

        Ok(())
    }

    pub fn chunk(&self, id: ChunkId) -> &ChunkAsset {
        &self.chunks[id.index()]
    }

    pub fn bond(&self, id: BondId) -> &BondAsset {
        &self.bonds[id.index()]
    }

    pub fn descendants(&self, root: ChunkId) -> Vec<ChunkId> {
        let mut queue = VecDeque::from([root]);
        let mut out = Vec::new();

        while let Some(id) = queue.pop_front() {
            out.push(id);
            queue.extend(self.chunk(id).children.iter().copied());
        }

        out
    }

    pub fn support_descendants(&self, root: ChunkId) -> Vec<ChunkId> {
        self.descendants(root)
            .into_iter()
            .filter(|chunk_id| self.chunk(*chunk_id).support_node)
            .collect()
    }

    pub fn fixed_support_chunks(&self) -> Vec<ChunkId> {
        self.support_chunks
            .iter()
            .copied()
            .filter(|chunk_id| self.chunk(*chunk_id).support.is_anchor())
            .collect()
    }

    pub fn support_chunk_count(&self) -> usize {
        self.support_chunks.len()
    }
}
