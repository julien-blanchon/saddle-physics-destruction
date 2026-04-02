use std::cmp::Reverse;

use bevy::prelude::*;

use crate::{
    ApplyDestructionDamage, ChunkGroupDetached, ChunkId, CleanupPolicy, Destructible,
    DestructionAssetHandle, DestructionConfig, DestructionDiagnostics, DestructionStarted,
    DestructionState, DestructionViewers, FinalDestructionOccurred, FractureBias, FracturedAsset,
    Fragment, FragmentLifetime, FragmentSpawnData, InitialVelocity, LodStrategy, MaterialHint,
    RootVisualMode, SupportAnchors,
    components::{DestructionRuntime, LastDamageSnapshot, PendingDamage, PendingDetachedGroup},
    damage,
};

pub(crate) fn ensure_runtime_initialized(
    mut commands: Commands,
    assets: Res<Assets<FracturedAsset>>,
    roots: Query<(Entity, &DestructionAssetHandle, Option<&DestructionRuntime>)>,
) {
    for (entity, asset_handle, runtime) in &roots {
        let Some(asset) = assets.get(&asset_handle.0) else {
            continue;
        };

        let needs_init = runtime.is_none_or(|runtime| {
            runtime.bond_health.len() != asset.bonds.len()
                || runtime.chunk_damage.len() != asset.chunks.len()
                || runtime.detached_chunks.len() != asset.chunks.len()
        });

        if needs_init {
            commands.entity(entity).insert((
                DestructionRuntime::from_asset(asset),
                DestructionState::default(),
            ));
        }
    }
}

pub(crate) fn process_damage_messages(
    assets: Res<Assets<FracturedAsset>>,
    mut reader: MessageReader<ApplyDestructionDamage>,
    mut writer: MessageWriter<DestructionStarted>,
    mut roots: Query<(
        Entity,
        &DestructionAssetHandle,
        &GlobalTransform,
        &mut DestructionRuntime,
    )>,
) {
    for message in reader.read().cloned() {
        let Ok((entity, asset_handle, root_transform, mut runtime)) = roots.get_mut(message.target)
        else {
            continue;
        };
        let Some(asset) = assets.get(&asset_handle.0) else {
            continue;
        };

        let local_from_world = root_transform.affine().inverse();

        let profile = damage::DamageProfile {
            origin: local_from_world.transform_point3(message.origin),
            direction: local_from_world.transform_vector3(message.direction),
            magnitude: message.magnitude.max(0.0),
            radius: message.radius.max(0.001),
            kind: message.kind,
            falloff: message.falloff,
        };

        if !runtime.started {
            runtime.started = true;
            writer.write(DestructionStarted {
                source: entity,
                world_position: message.origin,
                material_hint: dominant_material(asset, &asset.support_chunks),
                energy: message.magnitude,
            });
        }

        runtime.last_damage = Some(LastDamageSnapshot {
            origin: message.origin,
            direction: message.direction,
            radius: message.radius,
            energy: message.magnitude.max(0.0),
            fracture_bias: message.fracture_bias,
        });

        runtime.pending_damage.push(PendingDamage {
            profile,
            world_origin: message.origin,
            world_direction: message.direction,
            energy: message.magnitude.max(0.0),
            fracture_bias: message.fracture_bias,
        });
    }
}

pub(crate) fn evaluate_accumulated_damage(
    assets: Res<Assets<FracturedAsset>>,
    mut roots: Query<(
        &DestructionAssetHandle,
        &mut DestructionRuntime,
        &mut DestructionState,
    )>,
) {
    for (asset_handle, mut runtime, mut state) in &mut roots {
        if runtime.pending_damage.is_empty() {
            continue;
        }

        let Some(asset) = assets.get(&asset_handle.0) else {
            continue;
        };

        let mut topology_changed = false;
        let pending_damage = std::mem::take(&mut runtime.pending_damage);

        for pending in pending_damage {
            for support_chunk in asset.support_chunks.iter().copied() {
                if runtime.is_detached(support_chunk) {
                    continue;
                }
                let chunk = asset.chunk(support_chunk);
                let delta = damage::chunk_damage(pending.profile, chunk);
                runtime.chunk_damage[support_chunk.index()] += delta;

                if !chunk.tags.never_detach
                    && runtime.chunk_damage[support_chunk.index()] >= chunk.damage_threshold
                {
                    runtime.detached_chunks[support_chunk.index()] = true;
                    runtime.pending_groups.push(PendingDetachedGroup {
                        support_chunks: vec![support_chunk],
                        world_center: chunk.centroid,
                        energy: pending.energy,
                        material_hint: chunk.material_hint,
                        origin: pending.world_origin,
                        direction: pending.world_direction,
                        fracture_bias: pending.fracture_bias,
                        activation_chunks: None,
                        activation_cursor: 0,
                        detach_message_sent: false,
                    });
                    topology_changed = true;
                }
            }

            for bond in &asset.bonds {
                if runtime.is_detached(bond.chunks[0]) || runtime.is_detached(bond.chunks[1]) {
                    continue;
                }

                let bond_health = &mut runtime.bond_health[bond.id.index()];
                if *bond_health <= 0.0 {
                    continue;
                }

                let previous = *bond_health;
                *bond_health = (*bond_health - damage::bond_damage(pending.profile, bond)).max(0.0);
                if previous > 0.0 && *bond_health <= 0.0 {
                    topology_changed = true;
                }
            }
        }

        runtime.topology_dirty |= topology_changed;
        update_state(asset, &runtime, &mut state);
    }
}

pub(crate) fn evaluate_support_graphs(
    assets: Res<Assets<FracturedAsset>>,
    config: Res<DestructionConfig>,
    mut roots: Query<(
        &DestructionAssetHandle,
        &mut DestructionRuntime,
        &mut DestructionState,
        Option<&SupportAnchors>,
    )>,
) {
    if !config.enable_support_evaluation {
        return;
    }

    for (asset_handle, mut runtime, mut state, anchors_override) in &mut roots {
        if !runtime.topology_dirty {
            continue;
        }

        let Some(asset) = assets.get(&asset_handle.0) else {
            continue;
        };

        let anchors = anchors_override
            .map(|anchors| anchors.chunks.clone())
            .filter(|anchors| !anchors.is_empty())
            .unwrap_or_else(|| asset.fixed_support_chunks());

        for island in crate::graph::unsupported_islands(asset, &runtime, &anchors) {
            let material_hint = dominant_material(asset, &island);
            let center = crate::graph::group_centroid(asset, &island);

            for chunk in &island {
                runtime.detached_chunks[chunk.index()] = true;
            }

            let last_damage = runtime.last_damage.unwrap_or_default();
            runtime.pending_groups.push(PendingDetachedGroup {
                support_chunks: island,
                world_center: center,
                energy: last_damage.energy,
                material_hint,
                origin: last_damage.origin,
                direction: last_damage.direction,
                fracture_bias: last_damage.fracture_bias,
                activation_chunks: None,
                activation_cursor: 0,
                detach_message_sent: false,
            });
        }

        runtime.topology_dirty = false;
        runtime.total_support_evaluations += 1;
        update_state(asset, &runtime, &mut state);
    }
}

pub(crate) fn activate_pending_groups(
    mut commands: Commands,
    assets: Res<Assets<FracturedAsset>>,
    config: Res<DestructionConfig>,
    viewers: Res<DestructionViewers>,
    mut detached_writer: MessageWriter<ChunkGroupDetached>,
    mut final_writer: MessageWriter<FinalDestructionOccurred>,
    mut roots: Query<(
        Entity,
        Option<&Name>,
        &Destructible,
        &DestructionAssetHandle,
        &GlobalTransform,
        &mut DestructionRuntime,
        &mut DestructionState,
    )>,
) {
    let mut remaining_budget = config.max_chunk_spawns_per_frame;

    for (entity, name, destructible, asset_handle, root_transform, mut runtime, mut state) in
        &mut roots
    {
        let Some(asset) = assets.get(&asset_handle.0) else {
            continue;
        };

        if !runtime.pending_groups.is_empty() {
            let viewer_distance = nearest_viewer_distance(&viewers, root_transform.translation());
            let strategy = lod_strategy(&config, viewer_distance);
            let pending_groups = std::mem::take(&mut runtime.pending_groups);

            for mut group in pending_groups {
                if group.activation_chunks.is_none() {
                    group.activation_chunks = Some(select_activation_chunks(
                        asset,
                        &group.support_chunks,
                        strategy,
                        group.fracture_bias,
                    ));
                }

                let initial_velocity = compute_initial_velocity(&group, root_transform, &config);
                let planned_fragments = group.activation_chunks.as_ref().map_or(0, Vec::len);

                if !group.detach_message_sent {
                    detached_writer.write(ChunkGroupDetached {
                        source: entity,
                        chunk_ids: group.support_chunks.clone(),
                        fragment_count: planned_fragments,
                        world_position: root_transform.transform_point(group.world_center),
                        material_hint: group.material_hint,
                        impulse: initial_velocity,
                    });
                    group.detach_message_sent = true;
                    runtime.detached_group_count += 1;
                }

                if planned_fragments == 0 {
                    continue;
                }

                if remaining_budget == 0 {
                    runtime.pending_groups.push(group);
                    continue;
                }

                let activation_chunks = group.activation_chunks.clone().unwrap_or_default();

                while group.activation_cursor < activation_chunks.len() && remaining_budget > 0 {
                    let chunk_id = activation_chunks[group.activation_cursor];
                    let chunk = asset.chunk(chunk_id);
                    let fragment_transform = root_transform.mul_transform(chunk.local_transform);
                    let approx_size = chunk.half_extents.length() * 2.0;
                    commands.spawn((
                        Name::new(format!(
                            "{} Fragment {}",
                            name.map(Name::as_str).unwrap_or("Destructible"),
                            chunk.id.0
                        )),
                        Fragment {
                            source: entity,
                            primary_chunk: chunk.id,
                            chunk_count: asset.support_descendants(chunk.id).len().max(1) as u32,
                            fracture_level: chunk.fracture_level,
                            material_hint: chunk.material_hint,
                        },
                        FragmentLifetime {
                            remaining_secs: config.default_fragment_lifetime_secs,
                            fade_secs: config.fragment_fade_secs,
                            normalized_alpha: 1.0,
                        },
                        FragmentSpawnData {
                            chunk_ids: asset.support_descendants(chunk.id),
                            render: chunk.render.clone(),
                            collider: chunk.collider.clone(),
                            initial_velocity,
                            mass_hint: chunk.mass_hint,
                            approximate_size: approx_size,
                            world_center: fragment_transform.translation(),
                            material_hint: chunk.material_hint,
                        },
                        fragment_transform,
                    ));
                    remaining_budget -= 1;
                    group.activation_cursor += 1;
                    state.active_fragments += 1;
                    state.fracture_level = state.fracture_level.max(chunk.fracture_level);
                }

                if group.activation_cursor < activation_chunks.len() {
                    runtime.pending_groups.push(group);
                }
            }
        }

        if state.broken && !runtime.final_message_sent {
            runtime.final_message_sent = true;
            if matches!(destructible.visual_mode, RootVisualMode::HideWhenBroken) {
                commands.entity(entity).insert(Visibility::Hidden);
            }
            final_writer.write(FinalDestructionOccurred {
                source: entity,
                world_position: root_transform.translation(),
                detached_groups: runtime.detached_group_count,
                chunk_count: asset.support_chunks.len(),
                material_hint: dominant_material(asset, &asset.support_chunks),
            });
        }
    }
}

pub(crate) fn update_fragment_lifetimes(
    time: Res<Time>,
    mut fragments: Query<&mut FragmentLifetime>,
) {
    let delta = time.delta_secs();
    for mut lifetime in &mut fragments {
        lifetime.remaining_secs -= delta;
        if lifetime.remaining_secs <= 0.0 {
            lifetime.normalized_alpha = 0.0;
            continue;
        }
        if lifetime.fade_secs > 0.0 && lifetime.remaining_secs <= lifetime.fade_secs {
            lifetime.normalized_alpha =
                (lifetime.remaining_secs / lifetime.fade_secs).clamp(0.0, 1.0);
        } else {
            lifetime.normalized_alpha = 1.0;
        }
    }
}

pub(crate) fn cleanup_fragments(
    mut commands: Commands,
    config: Res<DestructionConfig>,
    viewers: Res<DestructionViewers>,
    mut diagnostics: ResMut<DestructionDiagnostics>,
    fragments: Query<(Entity, &FragmentLifetime, &FragmentSpawnData, &Transform)>,
) {
    let viewer = viewers.positions.first().copied();
    let mut survivors = Vec::new();

    for (entity, lifetime, spawn, transform) in &fragments {
        let too_old = lifetime.remaining_secs <= 0.0;
        let too_far = viewer.is_some_and(|viewer| {
            viewer.distance(transform.translation) > config.max_fragment_distance.max(0.001)
        });

        if too_old {
            commands.entity(entity).despawn();
            diagnostics.total_lifetime_trims += 1;
            continue;
        }
        if too_far {
            commands.entity(entity).despawn();
            diagnostics.total_distance_trims += 1;
            continue;
        }

        let distance = viewer
            .map(|viewer| viewer.distance(transform.translation))
            .unwrap_or(0.0);
        survivors.push((
            entity,
            lifetime.remaining_secs,
            spawn.approximate_size,
            distance,
        ));
    }

    if survivors.len() > config.fragment_budget {
        match config.cleanup_policy {
            CleanupPolicy::OldestFirst => {
                survivors.sort_by(|left, right| left.1.total_cmp(&right.1))
            }
            CleanupPolicy::SmallestFirst => {
                survivors.sort_by(|left, right| left.2.total_cmp(&right.2))
            }
            CleanupPolicy::FarthestFirst => {
                survivors.sort_by(|left, right| left.3.total_cmp(&right.3))
            }
        }

        for (entity, ..) in survivors
            .iter()
            .take(survivors.len() - config.fragment_budget)
        {
            commands.entity(*entity).despawn();
            diagnostics.total_budget_trims += 1;
        }
    }
}

pub(crate) fn sync_root_states(
    mut commands: Commands,
    fragments: Query<&Fragment>,
    mut roots: Query<(
        Entity,
        &Destructible,
        &mut DestructionState,
        Option<&Visibility>,
    )>,
) {
    let mut active_fragments = std::collections::HashMap::<Entity, u32>::new();
    for fragment in &fragments {
        *active_fragments.entry(fragment.source).or_default() += 1;
    }

    for (entity, destructible, mut state, visibility) in &mut roots {
        state.active_fragments = active_fragments.remove(&entity).unwrap_or(0);

        let should_hide = match destructible.visual_mode {
            RootVisualMode::KeepVisible => false,
            RootVisualMode::HideOnFirstDetach => state.detached_chunks > 0,
            RootVisualMode::HideWhenBroken => state.broken,
        };

        if should_hide && visibility.is_none_or(|visibility| *visibility != Visibility::Hidden) {
            commands.entity(entity).insert(Visibility::Hidden);
        }
    }
}

pub(crate) fn publish_diagnostics(
    mut diagnostics: ResMut<DestructionDiagnostics>,
    fragments: Query<Entity, With<Fragment>>,
    roots: Query<&DestructionRuntime>,
) {
    diagnostics.active_fragments = fragments.iter().count();
    diagnostics.pending_groups = roots
        .iter()
        .map(|runtime| runtime.pending_groups.len())
        .sum();
    diagnostics.total_detached_groups = roots
        .iter()
        .map(|runtime| runtime.detached_group_count as u64)
        .sum();
    diagnostics.total_support_evaluations = roots
        .iter()
        .map(|runtime| runtime.total_support_evaluations)
        .sum();
}

fn update_state(
    asset: &FracturedAsset,
    runtime: &DestructionRuntime,
    state: &mut DestructionState,
) {
    let detached = asset
        .support_chunks
        .iter()
        .filter(|chunk_id| runtime.is_detached(**chunk_id))
        .count();
    let total = asset.support_chunks.len().max(1) as f32;
    let accumulated_preview = asset
        .support_chunks
        .iter()
        .map(|chunk_id| {
            let chunk = asset.chunk(*chunk_id);
            damage::normalized_damage(
                runtime.chunk_damage[chunk_id.index()],
                chunk.damage_threshold,
            )
        })
        .sum::<f32>()
        / total;

    state.normalized_damage = accumulated_preview.clamp(0.0, 1.0);
    state.detached_chunks = detached as u32;
    state.broken = detached >= asset.support_chunks.len();
}

fn nearest_viewer_distance(viewers: &DestructionViewers, position: Vec3) -> f32 {
    viewers
        .positions
        .iter()
        .map(|viewer| viewer.distance(position))
        .min_by(f32::total_cmp)
        .unwrap_or(0.0)
}

fn lod_strategy(config: &DestructionConfig, distance: f32) -> LodStrategy {
    config
        .distance_lod
        .iter()
        .find(|band| distance <= band.max_distance)
        .map(|band| band.strategy)
        .unwrap_or(LodStrategy::Full)
}

fn dominant_material(asset: &FracturedAsset, group: &[ChunkId]) -> MaterialHint {
    let mut counts = std::collections::HashMap::<MaterialHint, usize>::new();
    for chunk_id in group {
        *counts
            .entry(asset.chunk(*chunk_id).material_hint)
            .or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(material, _)| material)
        .unwrap_or(MaterialHint::Generic)
}

fn compute_initial_velocity(
    group: &PendingDetachedGroup,
    root_transform: &GlobalTransform,
    config: &DestructionConfig,
) -> InitialVelocity {
    let direction = if config.inherit_velocity {
        group.direction.normalize_or_zero()
    } else {
        Vec3::ZERO
    };
    let origin = root_transform.transform_point(group.world_center);
    let radial = (origin - group.origin).normalize_or_zero();
    InitialVelocity {
        linear: (direction * 0.8 + radial * 0.6) * group.energy,
        angular: Vec3::new(radial.z, direction.x, radial.x) * (group.energy * 0.4),
    }
}

fn select_activation_chunks(
    asset: &FracturedAsset,
    support_group: &[ChunkId],
    strategy: LodStrategy,
    fracture_bias: FractureBias,
) -> Vec<ChunkId> {
    if matches!(strategy, LodStrategy::EventOnly) {
        return Vec::new();
    }

    let minimum_leaf_count = match strategy {
        LodStrategy::Full => usize::MAX,
        LodStrategy::Clustered { minimum_leaf_count } => minimum_leaf_count,
        LodStrategy::EventOnly => usize::MAX,
    };

    if matches!(fracture_bias, FractureBias::Fine) || matches!(strategy, LodStrategy::Full) {
        return support_group.to_vec();
    }

    let group_members = support_group
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let mut parents = asset
        .chunks
        .iter()
        .filter(|chunk| !chunk.support_node && !chunk.children.is_empty())
        .map(|chunk| {
            let leaf_descendants = asset.support_descendants(chunk.id);
            (chunk.id, leaf_descendants)
        })
        .filter(|(_, leaf_descendants)| {
            leaf_descendants.len() >= minimum_leaf_count
                && leaf_descendants
                    .iter()
                    .all(|leaf| group_members.contains(leaf))
        })
        .collect::<Vec<_>>();

    parents.sort_by_key(|(_, descendants)| Reverse(descendants.len()));
    let mut covered = std::collections::HashSet::new();
    let mut selected = Vec::new();

    for (parent, descendants) in parents {
        if descendants
            .iter()
            .any(|chunk_id| covered.contains(chunk_id))
        {
            continue;
        }
        if matches!(fracture_bias, FractureBias::Balanced) && descendants.len() < 4 {
            continue;
        }
        covered.extend(descendants);
        selected.push(parent);
    }

    for leaf in support_group {
        if !covered.contains(leaf) {
            selected.push(*leaf);
        }
    }

    selected
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
