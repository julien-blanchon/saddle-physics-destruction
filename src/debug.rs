use bevy::prelude::*;

use crate::{
    DestructionAssetHandle, DestructionDebugConfig, FracturedAsset, components::DestructionRuntime,
    graph,
};

pub(crate) fn draw_debug_gizmos(
    debug: Res<DestructionDebugConfig>,
    assets: Res<Assets<FracturedAsset>>,
    roots: Query<(&DestructionAssetHandle, &DestructionRuntime, &Transform)>,
    mut gizmos: Gizmos,
) {
    if !debug.draw_support_graph
        && !debug.draw_support_anchors
        && !debug.draw_unsupported_groups
        && !debug.draw_last_damage
    {
        return;
    }

    for (asset_handle, runtime, transform) in &roots {
        let Some(asset) = assets.get(&asset_handle.0) else {
            continue;
        };

        if debug.draw_support_graph {
            for bond in &asset.bonds {
                if runtime.bond_health[bond.id.index()] <= 0.0 {
                    continue;
                }
                let left = transform.transform_point(asset.chunk(bond.chunks[0]).centroid);
                let right = transform.transform_point(asset.chunk(bond.chunks[1]).centroid);
                let health =
                    (runtime.bond_health[bond.id.index()] / bond.health.max(0.001)).clamp(0.0, 1.0);
                gizmos.line(left, right, Color::srgb(1.0 - health, health, 0.1));
            }
        }

        if debug.draw_support_anchors {
            for anchor in asset.fixed_support_chunks() {
                let position = transform.transform_point(asset.chunk(anchor).centroid);
                gizmos.cross(position, 0.12, Color::srgb(0.2, 0.9, 0.3));
            }
        }

        if debug.draw_unsupported_groups {
            for island in graph::unsupported_islands(asset, runtime, &asset.fixed_support_chunks())
            {
                let center = transform.transform_point(graph::group_centroid(asset, &island));
                gizmos.sphere(center, 0.18, Color::srgb(0.95, 0.45, 0.12));
            }
        }

        if debug.draw_last_damage {
            if let Some(last_damage) = runtime.last_damage {
                gizmos.sphere(
                    last_damage.origin,
                    last_damage.radius.max(0.05),
                    Color::srgba(1.0, 0.2, 0.2, 0.35),
                );
                gizmos.ray(
                    last_damage.origin,
                    last_damage.direction.normalize_or_zero() * 0.75,
                    Color::srgb(1.0, 0.2, 0.2),
                );
            }
        }
    }
}
