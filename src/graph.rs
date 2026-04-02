use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::{ChunkId, FracturedAsset, components::DestructionRuntime};

pub fn unsupported_islands(
    asset: &FracturedAsset,
    runtime: &DestructionRuntime,
    anchors: &[ChunkId],
) -> Vec<Vec<ChunkId>> {
    let adjacency = build_support_adjacency(asset, runtime);

    if anchors.is_empty() {
        let mut components = support_components(asset, runtime, &adjacency);
        if components.len() <= 1 {
            return Vec::new();
        }

        let primary_index = components
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                group_mass(asset, left)
                    .total_cmp(&group_mass(asset, right))
                    .then_with(|| min_chunk_id(right).cmp(&min_chunk_id(left)))
            })
            .map(|(index, _)| index)
            .unwrap_or(0);

        components.swap_remove(primary_index);
        return components;
    }

    let mut supported = HashSet::new();
    let mut frontier = VecDeque::new();

    for anchor in anchors
        .iter()
        .copied()
        .filter(|chunk_id| !runtime.is_detached(*chunk_id))
    {
        supported.insert(anchor);
        frontier.push_back(anchor);
    }

    while let Some(current) = frontier.pop_front() {
        for neighbor in adjacency[current.index()].iter().copied() {
            if supported.insert(neighbor) {
                frontier.push_back(neighbor);
            }
        }
    }

    let mut visited = HashSet::new();
    let mut islands = Vec::new();

    for support_chunk in asset.support_chunks.iter().copied() {
        if runtime.is_detached(support_chunk)
            || supported.contains(&support_chunk)
            || !visited.insert(support_chunk)
        {
            continue;
        }

        let mut island = Vec::new();
        let mut queue = VecDeque::from([support_chunk]);

        while let Some(current) = queue.pop_front() {
            island.push(current);
            for neighbor in adjacency[current.index()].iter().copied() {
                if supported.contains(&neighbor) || runtime.is_detached(neighbor) {
                    continue;
                }
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        islands.push(island);
    }

    islands
}

fn support_components(
    asset: &FracturedAsset,
    runtime: &DestructionRuntime,
    adjacency: &[Vec<ChunkId>],
) -> Vec<Vec<ChunkId>> {
    let mut visited = HashSet::new();
    let mut components = Vec::new();

    for support_chunk in asset.support_chunks.iter().copied() {
        if runtime.is_detached(support_chunk) || !visited.insert(support_chunk) {
            continue;
        }

        let mut component = Vec::new();
        let mut queue = VecDeque::from([support_chunk]);

        while let Some(current) = queue.pop_front() {
            component.push(current);
            for neighbor in adjacency[current.index()].iter().copied() {
                if runtime.is_detached(neighbor) {
                    continue;
                }
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        components.push(component);
    }

    components
}

fn group_mass(asset: &FracturedAsset, group: &[ChunkId]) -> f32 {
    group
        .iter()
        .map(|chunk_id| asset.chunk(*chunk_id).mass_hint.max(0.001))
        .sum()
}

fn min_chunk_id(group: &[ChunkId]) -> ChunkId {
    group
        .iter()
        .copied()
        .min()
        .unwrap_or(ChunkId::new(u32::MAX))
}

pub fn group_centroid(asset: &FracturedAsset, group: &[ChunkId]) -> Vec3 {
    let mut total = Vec3::ZERO;
    let mut weight_sum = 0.0;

    for chunk_id in group {
        let chunk = asset.chunk(*chunk_id);
        let weight = chunk.mass_hint.max(0.01);
        total += chunk.centroid * weight;
        weight_sum += weight;
    }

    if weight_sum <= f32::EPSILON {
        Vec3::ZERO
    } else {
        total / weight_sum
    }
}

pub fn build_support_adjacency(
    asset: &FracturedAsset,
    runtime: &DestructionRuntime,
) -> Vec<Vec<ChunkId>> {
    let mut adjacency = vec![Vec::new(); asset.chunks.len()];

    for bond in &asset.bonds {
        if runtime
            .bond_health
            .get(bond.id.index())
            .copied()
            .unwrap_or_default()
            <= 0.0
        {
            continue;
        }

        let [left, right] = bond.chunks;
        if runtime.is_detached(left) || runtime.is_detached(right) {
            continue;
        }

        adjacency[left.index()].push(right);
        adjacency[right.index()].push(left);
    }

    adjacency
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
