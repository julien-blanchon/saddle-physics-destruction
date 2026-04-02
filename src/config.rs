use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum CleanupPolicy {
    #[default]
    OldestFirst,
    SmallestFirst,
    FarthestFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum LodStrategy {
    #[default]
    Full,
    Clustered {
        minimum_leaf_count: usize,
    },
    EventOnly,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct DistanceLodBand {
    pub max_distance: f32,
    pub strategy: LodStrategy,
}

impl Default for DistanceLodBand {
    fn default() -> Self {
        Self {
            max_distance: f32::MAX,
            strategy: LodStrategy::Full,
        }
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct DestructionConfig {
    pub fragment_budget: usize,
    pub cleanup_policy: CleanupPolicy,
    pub default_fragment_lifetime_secs: f32,
    pub fragment_fade_secs: f32,
    pub max_fragment_distance: f32,
    pub max_chunk_spawns_per_frame: usize,
    pub enable_support_evaluation: bool,
    pub inherit_velocity: bool,
    pub distance_lod: Vec<DistanceLodBand>,
}

impl Default for DestructionConfig {
    fn default() -> Self {
        Self {
            fragment_budget: 256,
            cleanup_policy: CleanupPolicy::OldestFirst,
            default_fragment_lifetime_secs: 8.0,
            fragment_fade_secs: 1.25,
            max_fragment_distance: 80.0,
            max_chunk_spawns_per_frame: 48,
            enable_support_evaluation: true,
            inherit_velocity: true,
            distance_lod: vec![DistanceLodBand::default()],
        }
    }
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct DestructionViewers {
    pub positions: Vec<Vec3>,
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct DestructionDiagnostics {
    pub active_fragments: usize,
    pub pending_groups: usize,
    pub total_detached_groups: u64,
    pub total_support_evaluations: u64,
    pub total_budget_trims: u64,
    pub total_distance_trims: u64,
    pub total_lifetime_trims: u64,
}

#[derive(Resource, Debug, Clone, Reflect, Default)]
#[reflect(Resource)]
pub struct DestructionDebugConfig {
    pub draw_support_graph: bool,
    pub draw_support_anchors: bool,
    pub draw_unsupported_groups: bool,
    pub draw_last_damage: bool,
}
