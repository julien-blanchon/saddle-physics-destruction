#![doc = include_str!("../README.md")]

use bevy::ecs::{intern::Interned, schedule::ScheduleLabel};
use bevy::gizmos::config::GizmoConfigStore;
use bevy::prelude::*;

mod asset;
mod authoring;
mod components;
mod config;
mod damage;
mod debug;
mod graph;
mod ids;
mod messages;
mod render;
mod systems;

pub use asset::{
    BondAsset, ChunkAsset, ChunkTags, ColliderSource, FractureGenerator, FractureMetadata,
    FracturedAsset, FragmentRenderData, MaterialHint, SupportKind,
};
pub use authoring::{
    CuboidAnchorPreset, CuboidFractureBuilder, ThinSurfaceAnchorPreset, ThinSurfaceFractureBuilder,
};
pub use components::{
    Destructible, DestructionAssetHandle, DestructionState, Fragment, FragmentLifetime,
    FragmentSpawnData, InitialVelocity, RootVisualMode, SupportAnchors,
};
pub use config::{
    CleanupPolicy, DestructionConfig, DestructionDebugConfig, DestructionDiagnostics,
    DestructionViewers, DistanceLodBand, LodStrategy,
};
pub use ids::{BondId, ChunkId};
pub use messages::{
    ApplyDestructionDamage, ChunkGroupDetached, DamageKind, DestructionStarted, FalloffCurve,
    FinalDestructionOccurred, FractureBias,
};
pub use render::build_fragment_mesh;

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum DestructionSystems {
    AccumulateDamage,
    EvaluateBonds,
    EvaluateSupport,
    ActivateFragments,
    CleanupDebris,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct DestructionPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl DestructionPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for DestructionPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for DestructionPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        app.init_asset::<FracturedAsset>()
            .register_asset_reflect::<FracturedAsset>()
            .init_resource::<DestructionConfig>()
            .init_resource::<DestructionViewers>()
            .init_resource::<DestructionDiagnostics>()
            .init_resource::<DestructionDebugConfig>()
            .add_message::<ApplyDestructionDamage>()
            .add_message::<DestructionStarted>()
            .add_message::<ChunkGroupDetached>()
            .add_message::<FinalDestructionOccurred>()
            .register_type::<ApplyDestructionDamage>()
            .register_type::<BondAsset>()
            .register_type::<BondId>()
            .register_type::<ChunkAsset>()
            .register_type::<ChunkId>()
            .register_type::<ChunkTags>()
            .register_type::<ChunkGroupDetached>()
            .register_type::<CleanupPolicy>()
            .register_type::<ColliderSource>()
            .register_type::<Destructible>()
            .register_type::<DestructionAssetHandle>()
            .register_type::<DestructionConfig>()
            .register_type::<DestructionDebugConfig>()
            .register_type::<DestructionDiagnostics>()
            .register_type::<DestructionState>()
            .register_type::<DestructionStarted>()
            .register_type::<DestructionViewers>()
            .register_type::<DistanceLodBand>()
            .register_type::<FalloffCurve>()
            .register_type::<FinalDestructionOccurred>()
            .register_type::<Fragment>()
            .register_type::<FragmentLifetime>()
            .register_type::<FragmentRenderData>()
            .register_type::<FragmentSpawnData>()
            .register_type::<FracturedAsset>()
            .register_type::<FractureBias>()
            .register_type::<FractureGenerator>()
            .register_type::<FractureMetadata>()
            .register_type::<InitialVelocity>()
            .register_type::<LodStrategy>()
            .register_type::<MaterialHint>()
            .register_type::<RootVisualMode>()
            .register_type::<SupportAnchors>()
            .register_type::<SupportKind>()
            .configure_sets(
                self.update_schedule,
                (
                    DestructionSystems::AccumulateDamage,
                    DestructionSystems::EvaluateBonds,
                    DestructionSystems::EvaluateSupport,
                    DestructionSystems::ActivateFragments,
                    DestructionSystems::CleanupDebris,
                )
                    .chain(),
            )
            .add_systems(self.activate_schedule, systems::ensure_runtime_initialized)
            .add_systems(
                self.update_schedule,
                systems::ensure_runtime_initialized.before(DestructionSystems::AccumulateDamage),
            )
            .add_systems(
                self.update_schedule,
                systems::process_damage_messages.in_set(DestructionSystems::AccumulateDamage),
            )
            .add_systems(
                self.update_schedule,
                systems::evaluate_accumulated_damage.in_set(DestructionSystems::EvaluateBonds),
            )
            .add_systems(
                self.update_schedule,
                systems::evaluate_support_graphs.in_set(DestructionSystems::EvaluateSupport),
            )
            .add_systems(
                self.update_schedule,
                systems::activate_pending_groups.in_set(DestructionSystems::ActivateFragments),
            )
            .add_systems(
                self.update_schedule,
                (
                    systems::update_fragment_lifetimes,
                    systems::cleanup_fragments,
                    systems::sync_root_states,
                    systems::publish_diagnostics,
                )
                    .chain()
                    .in_set(DestructionSystems::CleanupDebris),
            )
            .add_systems(
                self.update_schedule,
                debug::draw_debug_gizmos
                    .after(DestructionSystems::CleanupDebris)
                    .run_if(resource_exists::<GizmoConfigStore>),
            );
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
