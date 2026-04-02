use bevy::prelude::*;

use crate::{
    ChunkId, ColliderSource, FractureBias, FracturedAsset, FragmentRenderData, MaterialHint,
    damage::DamageProfile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum RootVisualMode {
    KeepVisible,
    #[default]
    HideOnFirstDetach,
    HideWhenBroken,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Destructible {
    pub visual_mode: RootVisualMode,
}

impl Default for Destructible {
    fn default() -> Self {
        Self {
            visual_mode: RootVisualMode::HideOnFirstDetach,
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct DestructionAssetHandle(pub Handle<FracturedAsset>);

#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct SupportAnchors {
    pub chunks: Vec<ChunkId>,
}

#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct DestructionState {
    pub normalized_damage: f32,
    pub fracture_level: u8,
    pub broken: bool,
    pub detached_chunks: u32,
    pub active_fragments: u32,
}

#[derive(Debug, Clone, Copy, Default, Reflect)]
pub struct InitialVelocity {
    pub linear: Vec3,
    pub angular: Vec3,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Fragment {
    pub source: Entity,
    pub primary_chunk: ChunkId,
    pub chunk_count: u32,
    pub fracture_level: u8,
    pub material_hint: MaterialHint,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct FragmentLifetime {
    pub remaining_secs: f32,
    pub fade_secs: f32,
    pub normalized_alpha: f32,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct FragmentSpawnData {
    pub chunk_ids: Vec<ChunkId>,
    pub render: FragmentRenderData,
    pub collider: Option<ColliderSource>,
    pub initial_velocity: InitialVelocity,
    pub mass_hint: f32,
    pub approximate_size: f32,
    pub world_center: Vec3,
    pub material_hint: MaterialHint,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingDetachedGroup {
    pub support_chunks: Vec<ChunkId>,
    pub world_center: Vec3,
    pub energy: f32,
    pub material_hint: MaterialHint,
    pub origin: Vec3,
    pub direction: Vec3,
    pub fracture_bias: FractureBias,
    pub activation_chunks: Option<Vec<ChunkId>>,
    pub activation_cursor: usize,
    pub detach_message_sent: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PendingDamage {
    pub profile: DamageProfile,
    pub world_origin: Vec3,
    pub world_direction: Vec3,
    pub energy: f32,
    pub fracture_bias: FractureBias,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LastDamageSnapshot {
    pub origin: Vec3,
    pub direction: Vec3,
    pub radius: f32,
    pub energy: f32,
    pub fracture_bias: FractureBias,
}

#[derive(Component, Debug, Clone, Default)]
pub(crate) struct DestructionRuntime {
    pub bond_health: Vec<f32>,
    pub chunk_damage: Vec<f32>,
    pub detached_chunks: Vec<bool>,
    pub topology_dirty: bool,
    pub started: bool,
    pub final_message_sent: bool,
    pub pending_groups: Vec<PendingDetachedGroup>,
    pub pending_damage: Vec<PendingDamage>,
    pub detached_group_count: u32,
    pub total_support_evaluations: u64,
    pub last_damage: Option<LastDamageSnapshot>,
}

impl DestructionRuntime {
    pub(crate) fn from_asset(asset: &FracturedAsset) -> Self {
        Self {
            bond_health: asset.bonds.iter().map(|bond| bond.health).collect(),
            chunk_damage: vec![0.0; asset.chunks.len()],
            detached_chunks: vec![false; asset.chunks.len()],
            topology_dirty: false,
            started: false,
            final_message_sent: false,
            pending_groups: Vec::new(),
            pending_damage: Vec::new(),
            detached_group_count: 0,
            total_support_evaluations: 0,
            last_damage: None,
        }
    }

    pub(crate) fn is_detached(&self, chunk_id: ChunkId) -> bool {
        self.detached_chunks
            .get(chunk_id.index())
            .copied()
            .unwrap_or(false)
    }
}
