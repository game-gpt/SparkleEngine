//! 网格：POSITION / NORMAL / TEXCOORD_0 / JOINTS_0 / WEIGHTS_0 → 三角列表。

use spark_renderer::SkinnedVertex;
use spark_types::Color;

use crate::import::{GltfError, ImportedMesh, aabb_from_positions, default_color};

pub(crate) fn import_meshes(gltf: &gltf::Gltf, buffers: &[Vec<u8>]) -> Result<Vec<ImportedMesh>, GltfError> {
    let get = |buffer: gltf::Buffer| buffers.get(buffer.index()).map(|b| b.as_slice());
    let mut out = Vec::new();
    for mesh in gltf.meshes() {
        let name = mesh.name().unwrap_or("mesh").to_string();
        for (pi, primitive) in mesh.primitives().enumerate() {
            let reader = primitive.reader(get);
            let positions: Vec<[f32; 3]> =
                reader.read_positions().ok_or_else(|| GltfError::invalid(format!("missing_position:{name}#{pi}")))?.collect();
            if positions.is_empty() {
                continue;
            }
            let n = positions.len();
            let normals: Vec<[f32; 3]> = match reader.read_normals() {
                Some(it) => it.collect(),
                None => vec![[0.0, 1.0, 0.0]; n],
            };
            let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
                Some(tc) => tc.into_f32().collect(),
                None => vec![[0.0, 0.0]; n],
            };
            let colors: Vec<Color> = match reader.read_colors(0) {
                Some(c) => c.into_rgba_f32().map(|rgba| Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3])).collect(),
                None => vec![default_color(); n],
            };
            let joints: Vec<[u32; 4]> = match reader.read_joints(0) {
                Some(j) => j.into_u16().map(|j| [j[0] as u32, j[1] as u32, j[2] as u32, j[3] as u32]).collect(),
                None => vec![[0, 0, 0, 0]; n],
            };
            let weights: Vec<[f32; 4]> = match reader.read_weights(0) {
                Some(w) => w.into_f32().map(normalize_weights).collect(),
                None => vec![[1.0, 0.0, 0.0, 0.0]; n],
            };
            if normals.len() != n || uvs.len() != n || colors.len() != n || joints.len() != n || weights.len() != n {
                return Err(GltfError::invalid(format!("attribute_len_mismatch:{name}#{pi}")));
            }

            let mut verts: Vec<SkinnedVertex> =
                (0..n).map(|i| SkinnedVertex::new(positions[i], normals[i], uvs[i], colors[i], joints[i], weights[i])).collect();

            if let Some(indices) = reader.read_indices() {
                let idx: Vec<u32> = indices.into_u32().collect();
                let mut expanded = Vec::with_capacity(idx.len());
                for i in idx {
                    let i = i as usize;
                    if i >= verts.len() {
                        return Err(GltfError::invalid(format!("index_out_of_bounds:{name}#{pi}")));
                    }
                    expanded.push(verts[i]);
                }
                verts = expanded;
            }

            let aabb = aabb_from_positions(&positions);
            let prim_name = if pi == 0 { name.clone() } else { format!("{name}#{pi}") };
            out.push(ImportedMesh { name: prim_name, vertices: verts, local_aabb: aabb });
        }
    }
    Ok(out)
}

fn normalize_weights(w: [f32; 4]) -> [f32; 4] {
    let sum = w[0] + w[1] + w[2] + w[3];
    if sum.abs() < 1e-8 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    [w[0] / sum, w[1] / sum, w[2] / sum, w[3] / sum]
}
