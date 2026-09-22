//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_gltf::*;

/// 最小三角网格（无蒙皮）：验证 POSITION / indices 展开。
#[test]
fn import_static_triangle() {
    // 顶点：(0,0,0) (1,0,0) (0,1,0)；索引 0,1,2
    let bin: Vec<u8> = {
        let mut v = Vec::new();
        for f in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            v.extend_from_slice(&f.to_le_bytes());
        }
        for i in [0u16, 1, 2] {
            v.extend_from_slice(&i.to_le_bytes());
        }
        v
    };
    let b64 = base64_encode(&bin);
    let json = format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "scenes": [{{"nodes": [0]}}],
  "nodes": [{{"mesh": 0, "name": "tri"}}],
  "meshes": [{{"name": "tri", "primitives": [{{"attributes": {{"POSITION": 0}}, "indices": 1}}]}}],
  "accessors": [
{{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "max": [1,1,0], "min": [0,0,0]}},
{{"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}}
  ],
  "bufferViews": [
{{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
{{"buffer": 0, "byteOffset": 36, "byteLength": 6}}
  ],
  "buffers": [{{"byteLength": {len}, "uri": "data:application/octet-stream;base64,{b64}"}}]
}}"#,
        len = bin.len(),
        b64 = b64
    );
    let asset = import_slice(json.as_bytes()).expect("import");
    assert_eq!(asset.meshes.len(), 1);
    assert_eq!(asset.meshes[0].vertices.len(), 3);
    assert!((asset.meshes[0].vertices[1].pos[0] - 1.0).abs() < 1e-5);
    assert!(asset.skeleton.is_none());
}

#[test]
fn import_two_bone_skinned() {
    let json = include_str!("../tests/fixtures/two_bone.gltf");
    let asset = import_slice(json.as_bytes()).expect("import skinned");
    let sk = asset.skeleton.expect("skeleton");
    assert_eq!(sk.joint_count(), 2);
    assert!(sk.find_joint("root").is_some());
    assert!(sk.find_joint("child").is_some());
    assert!(sk.find_socket("tip").is_some());
    assert!(!asset.meshes.is_empty());
    let v = &asset.meshes[0].vertices[0];
    assert!(v.weights[0] + v.weights[1] + v.weights[2] + v.weights[3] > 0.99);
}

#[test]
fn imported_skeleton_builds_rest_palette() {
    use spark_animator::{LocalPose, build_skin_palette, evaluate_pose, socket_world_position};

    let json = include_str!("../tests/fixtures/two_bone.gltf");
    let asset = import_slice(json.as_bytes()).expect("import");
    let sk = asset.skeleton.expect("skeleton");
    let pose = LocalPose::rest(&sk);
    let globals = evaluate_pose(&sk, &pose);
    let palette = build_skin_palette(&sk, &globals);
    assert_eq!(palette.len(), 2);
    let tip = socket_world_position(&sk, &globals, "tip").expect("tip");
    assert!((tip.y - 2.0).abs() < 1e-3, "tip should sit at y=2 in rest pose, got {}", tip.y);
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
