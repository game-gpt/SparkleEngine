//! 皮肤：骨架、挂点与动画通道。

use std::collections::{HashMap, HashSet};

use spark_animator::{
    Joint, JointTrack, QuatKey, Skeleton, SkinnedAnimationClip, Socket, Vec3Key, MAX_JOINTS,
};
use spark_geometry::{Mat4, Quat, Trs, Vec3};

use crate::import::GltfError;

/// 导入结果：骨架 + `skin joints 槽 → Skeleton.joints 下标` 映射。
pub(crate) struct SkinImport {
    pub skeleton: Skeleton,
    /// 长度 = skin.joints 数；`skin_to_out[skin_i] = 输出关节下标`。
    pub skin_to_out: Vec<u16>,
}

/// 取首个 skin；无 skin 则 `None`。
pub(crate) fn import_skeleton(
    gltf: &gltf::Gltf,
    buffers: &[Vec<u8>],
) -> Result<Option<SkinImport>, GltfError> {
    let Some(skin) = gltf.skins().next() else {
        return Ok(None);
    };
    let get = |buffer: gltf::Buffer| buffers.get(buffer.index()).map(|b| b.as_slice());

    let joint_nodes: Vec<gltf::Node> = skin.joints().collect();
    if joint_nodes.is_empty() {
        return Ok(None);
    }
    if joint_nodes.len() > MAX_JOINTS {
        return Err(GltfError::invalid(format!(
            "joint_count_exceeds_limit:{}>{MAX_JOINTS}",
            joint_nodes.len()
        )));
    }

    let ibm: Vec<Mat4> = match skin.reader(get).read_inverse_bind_matrices() {
        Some(iter) => iter.map(mat4_from_col_arrays).collect(),
        None => vec![Mat4::IDENTITY; joint_nodes.len()],
    };
    if ibm.len() != joint_nodes.len() {
        return Err(GltfError::invalid(
            "inverse_bind_matrices_len_mismatch",
        ));
    }

    let mut node_to_skin: HashMap<usize, usize> = HashMap::new();
    for (si, node) in joint_nodes.iter().enumerate() {
        node_to_skin.insert(node.index(), si);
    }

    let order = topo_joint_order(&joint_nodes)?;
    let mut skin_to_out = vec![0u16; joint_nodes.len()];
    for (out_i, &skin_i) in order.iter().enumerate() {
        skin_to_out[skin_i] = out_i as u16;
    }

    let mut joints = Vec::with_capacity(order.len());
    for &skin_i in &order {
        let node = &joint_nodes[skin_i];
        let name = node
            .name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("joint_{skin_i}"));
        let parent = parent_joint_out(gltf, node.index(), &node_to_skin, &skin_to_out);
        joints.push(Joint {
            name,
            parent,
            inverse_bind: ibm[skin_i],
            rest_local: node_to_trs(node),
        });
    }

    let sockets = import_sockets(gltf, &node_to_skin, &skin_to_out);
    Ok(Some(SkinImport {
        skeleton: Skeleton { joints, sockets },
        skin_to_out,
    }))
}

/// 导入动画；目标节点须落在骨架关节上。
pub(crate) fn import_animations(
    gltf: &gltf::Gltf,
    buffers: &[Vec<u8>],
    skin: &SkinImport,
) -> Result<Vec<SkinnedAnimationClip>, GltfError> {
    let Some(gltf_skin) = gltf.skins().next() else {
        return Ok(Vec::new());
    };
    let get = |buffer: gltf::Buffer| buffers.get(buffer.index()).map(|b| b.as_slice());

    let joint_nodes: Vec<gltf::Node> = gltf_skin.joints().collect();
    let mut node_to_skin: HashMap<usize, usize> = HashMap::new();
    for (si, node) in joint_nodes.iter().enumerate() {
        node_to_skin.insert(node.index(), si);
    }
    let node_to_joint: HashMap<usize, u16> = node_to_skin
        .iter()
        .map(|(&ni, &si)| (ni, skin.skin_to_out[si]))
        .collect();

    let mut clips = Vec::new();
    for anim in gltf.animations() {
        let name = anim.name().unwrap_or("clip").to_string();
        let mut tracks: HashMap<u16, JointTrack> = HashMap::new();
        let mut duration = 0.0f32;

        for channel in anim.channels() {
            let node = channel.target().node();
            let Some(&joint) = node_to_joint.get(&node.index()) else {
                continue;
            };
            let reader = channel.reader(get);
            let times: Vec<f32> = match reader.read_inputs() {
                Some(t) => t.collect(),
                None => continue,
            };
            if let Some(&t) = times.last() {
                duration = duration.max(t);
            }
            let track = tracks.entry(joint).or_insert_with(|| JointTrack {
                joint,
                translations: Vec::new(),
                rotations: Vec::new(),
                scales: Vec::new(),
            });
            match reader.read_outputs() {
                Some(gltf::animation::util::ReadOutputs::Translations(it)) => {
                    for (i, v) in it.enumerate() {
                        track.translations.push(Vec3Key {
                            time: times.get(i).copied().unwrap_or(0.0),
                            value: Vec3::new(v[0], v[1], v[2]),
                        });
                    }
                }
                Some(gltf::animation::util::ReadOutputs::Rotations(rots)) => {
                    for (i, v) in rots.into_f32().enumerate() {
                        track.rotations.push(QuatKey {
                            time: times.get(i).copied().unwrap_or(0.0),
                            value: Quat::new(v[0], v[1], v[2], v[3]).normalized(),
                        });
                    }
                }
                Some(gltf::animation::util::ReadOutputs::Scales(it)) => {
                    for (i, v) in it.enumerate() {
                        track.scales.push(Vec3Key {
                            time: times.get(i).copied().unwrap_or(0.0),
                            value: Vec3::new(v[0], v[1], v[2]),
                        });
                    }
                }
                _ => {}
            }
        }

        clips.push(SkinnedAnimationClip {
            name,
            duration,
            tracks: tracks.into_values().collect(),
        });
    }
    Ok(clips)
}

/// 将顶点 `JOINTS_0`（skin 槽）映射到输出骨架下标。
pub(crate) fn remap_mesh_joints(meshes: &mut [crate::import::ImportedMesh], skin_to_out: &[u16]) {
    for mesh in meshes {
        for v in &mut mesh.vertices {
            for k in 0..4 {
                let si = v.joints[k] as usize;
                if si < skin_to_out.len() {
                    v.joints[k] = u32::from(skin_to_out[si]);
                }
            }
        }
    }
}

fn import_sockets(
    gltf: &gltf::Gltf,
    node_to_skin: &HashMap<usize, usize>,
    skin_to_out: &[u16],
) -> Vec<Socket> {
    let mut sockets = Vec::new();
    for node in gltf.nodes() {
        if node_to_skin.contains_key(&node.index()) || node.mesh().is_some() {
            continue;
        }
        let Some(parent_node) = find_parent_node(gltf, node.index()) else {
            continue;
        };
        let Some(&skin_i) = node_to_skin.get(&parent_node) else {
            continue;
        };
        let name = node
            .name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("socket_{}", node.index()));
        sockets.push(Socket {
            name,
            parent_joint: skin_to_out[skin_i],
            local: node_to_trs(&node),
        });
    }
    sockets
}

fn find_parent_node(gltf: &gltf::Gltf, child: usize) -> Option<usize> {
    for node in gltf.nodes() {
        if node.children().any(|c| c.index() == child) {
            return Some(node.index());
        }
    }
    None
}

fn parent_joint_out(
    gltf: &gltf::Gltf,
    node_index: usize,
    node_to_skin: &HashMap<usize, usize>,
    skin_to_out: &[u16],
) -> Option<u16> {
    let mut cur = find_parent_node(gltf, node_index)?;
    loop {
        if let Some(&si) = node_to_skin.get(&cur) {
            return Some(skin_to_out[si]);
        }
        cur = find_parent_node(gltf, cur)?;
    }
}

fn topo_joint_order(joint_nodes: &[gltf::Node]) -> Result<Vec<usize>, GltfError> {
    let n = joint_nodes.len();
    let joint_set: HashSet<usize> = joint_nodes.iter().map(|j| j.index()).collect();
    let node_to_skin: HashMap<usize, usize> = joint_nodes
        .iter()
        .enumerate()
        .map(|(si, n)| (n.index(), si))
        .collect();

    let mut indeg = vec![0usize; n];
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (psi, parent) in joint_nodes.iter().enumerate() {
        collect_joint_children(
            parent,
            &joint_set,
            &node_to_skin,
            psi,
            &mut children,
            &mut indeg,
        );
    }

    let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut order = Vec::with_capacity(n);
    while let Some(i) = queue.pop() {
        order.push(i);
        for &c in &children[i] {
            indeg[c] -= 1;
            if indeg[c] == 0 {
                queue.push(c);
            }
        }
    }
    if order.len() != n {
        return Err(GltfError::invalid("joint_hierarchy_cycle"));
    }
    Ok(order)
}

fn collect_joint_children(
    parent: &gltf::Node,
    joint_set: &HashSet<usize>,
    node_to_skin: &HashMap<usize, usize>,
    parent_skin: usize,
    children: &mut [Vec<usize>],
    indeg: &mut [usize],
) {
    for child in parent.children() {
        if let Some(&csi) = node_to_skin.get(&child.index()) {
            if !children[parent_skin].contains(&csi) {
                children[parent_skin].push(csi);
                indeg[csi] += 1;
            }
        } else if !joint_set.contains(&child.index()) {
            collect_joint_children(
                &child,
                joint_set,
                node_to_skin,
                parent_skin,
                children,
                indeg,
            );
        }
    }
}

fn node_to_trs(node: &gltf::Node) -> Trs {
    let (t, r, s) = node.transform().decomposed();
    Trs::new(
        Vec3::new(t[0], t[1], t[2]),
        Quat::new(r[0], r[1], r[2], r[3]).normalized(),
        Vec3::new(s[0], s[1], s[2]),
    )
}

fn mat4_from_col_arrays(m: [[f32; 4]; 4]) -> Mat4 {
    let mut cols = [0.0f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            cols[c * 4 + r] = m[c][r];
        }
    }
    Mat4::from_cols(cols)
}
