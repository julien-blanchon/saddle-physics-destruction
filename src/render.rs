use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::FragmentRenderData;

pub fn build_fragment_mesh(render: &FragmentRenderData) -> Mesh {
    match render {
        FragmentRenderData::Cuboid { size, .. } => Mesh::from(Cuboid::new(size.x, size.y, size.z)),
        FragmentRenderData::ExtrudedConvex { polygon, depth, .. } => {
            build_extruded_convex_mesh(polygon, *depth)
        }
        FragmentRenderData::None => Mesh::from(Cuboid::new(0.001, 0.001, 0.001)),
    }
}

fn build_extruded_convex_mesh(polygon: &[Vec2], depth: f32) -> Mesh {
    let half_depth = depth * 0.5;
    let count = polygon.len().max(3);
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<u32>::new();

    let bounds = polygon.iter().fold(Rect::EMPTY, |rect, point| Rect {
        min: rect.min.min(*point),
        max: rect.max.max(*point),
    });
    let uv_size = (bounds.max - bounds.min).max(Vec2::splat(0.001));

    for point in polygon {
        positions.push([point.x, point.y, half_depth]);
        normals.push([0.0, 0.0, 1.0]);
        uvs.push([
            (point.x - bounds.min.x) / uv_size.x,
            (point.y - bounds.min.y) / uv_size.y,
        ]);
    }

    for point in polygon {
        positions.push([point.x, point.y, -half_depth]);
        normals.push([0.0, 0.0, -1.0]);
        uvs.push([
            (point.x - bounds.min.x) / uv_size.x,
            (point.y - bounds.min.y) / uv_size.y,
        ]);
    }

    for index in 1..count - 1 {
        indices.extend_from_slice(&[0, index as u32, index as u32 + 1]);
        indices.extend_from_slice(&[
            count as u32,
            count as u32 + index as u32 + 1,
            count as u32 + index as u32,
        ]);
    }

    for edge_index in 0..count {
        let next = (edge_index + 1) % count;
        let edge = polygon[next] - polygon[edge_index];
        let normal = Vec3::new(edge.y, -edge.x, 0.0).normalize_or_zero();
        let base = positions.len() as u32;
        let edge_length = edge.length().max(0.001);

        positions.extend_from_slice(&[
            [polygon[edge_index].x, polygon[edge_index].y, -half_depth],
            [polygon[next].x, polygon[next].y, -half_depth],
            [polygon[next].x, polygon[next].y, half_depth],
            [polygon[edge_index].x, polygon[edge_index].y, half_depth],
        ]);
        normals.extend_from_slice(&[[normal.x, normal.y, normal.z]; 4]);
        uvs.extend_from_slice(&[
            [0.0, 0.0],
            [edge_length, 0.0],
            [edge_length, depth.max(0.001)],
            [0.0, depth.max(0.001)],
        ]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}
