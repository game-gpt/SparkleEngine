//! 导入入口与错误类型。

use std::path::Path;

use spark_anim::{AnimationClip, Skeleton};
use spark_core::Color;
use spark_geometry::{Aabb3, Vec3};
use spark_renderer::SkinnedVertex;
use thiserror::Error;

use crate::mesh::import_meshes;
use crate::skin::{import_animations, import_skeleton, remap_mesh_joints};

#[derive(Debug, Error)]
pub enum GltfError {
    #[error(transparent)]
    Gltf(#[from] gltf::Error),
    #[error("IO：{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

/// 导入后的 CPU 网格（三角列表，已展开索引）。
#[derive(Debug, Clone)]
pub struct ImportedMesh {
    pub name: String,
    pub vertices: Vec<SkinnedVertex>,
    pub local_aabb: Aabb3,
}

/// glTF 文档的 Spark 侧投影。
#[derive(Debug, Clone)]
pub struct GltfAsset {
    pub skeleton: Option<Skeleton>,
    pub clips: Vec<AnimationClip>,
    pub meshes: Vec<ImportedMesh>,
}

/// 从字节导入（`.gltf` JSON 或 `.glb`）。
pub fn import_slice(bytes: &[u8]) -> Result<GltfAsset, GltfError> {
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let blob = gltf.blob.as_deref();
    import_doc(&gltf, blob, None)
}

/// 从路径导入；外置 `.bin` 相对路径按文件所在目录解析。
pub fn import_path(path: impl AsRef<Path>) -> Result<GltfAsset, GltfError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    let gltf = gltf::Gltf::from_slice(&bytes)?;
    let base = path.parent();
    let blob = gltf.blob.as_deref();
    import_doc(&gltf, blob, base)
}

fn import_doc(
    gltf: &gltf::Gltf,
    blob: Option<&[u8]>,
    base: Option<&Path>,
) -> Result<GltfAsset, GltfError> {
    let buffers = load_buffers(gltf, blob, base)?;
    let skin = import_skeleton(gltf, &buffers)?;
    let clips = if let Some(sk) = skin.as_ref() {
        import_animations(gltf, &buffers, sk)?
    } else {
        Vec::new()
    };
    let mut meshes = import_meshes(gltf, &buffers)?;
    let skeleton = if let Some(sk) = skin {
        remap_mesh_joints(&mut meshes, &sk.skin_to_out);
        Some(sk.skeleton)
    } else {
        None
    };
    Ok(GltfAsset {
        skeleton,
        clips,
        meshes,
    })
}

fn load_buffers(
    gltf: &gltf::Gltf,
    blob: Option<&[u8]>,
    base: Option<&Path>,
) -> Result<Vec<Vec<u8>>, GltfError> {
    let mut out = Vec::with_capacity(gltf.buffers().len());
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                let data = blob.ok_or_else(|| GltfError::Message("GLB 缺少 BIN 块".into()))?;
                out.push(data.to_vec());
            }
            gltf::buffer::Source::Uri(uri) => {
                if let Some(data) = uri.strip_prefix("data:") {
                    let b64 = data
                        .split(',')
                        .nth(1)
                        .ok_or_else(|| GltfError::Message("非法 data URI".into()))?;
                    out.push(decode_base64(b64)?);
                } else {
                    let base = base.ok_or_else(|| {
                        GltfError::Message("外置 buffer 需要文件路径导入".into())
                    })?;
                    let path = base.join(uri);
                    out.push(std::fs::read(path)?);
                }
            }
        }
    }
    Ok(out)
}

fn decode_base64(s: &str) -> Result<Vec<u8>, GltfError> {
    // 无额外依赖：标准库式手写解码器过重，用简易表。
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let s = s.as_bytes();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf = [0u8; 4];
    let mut n = 0;
    for &c in s {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let Some(v) = val(c) else {
            return Err(GltfError::Message("base64 非法字符".into()));
        };
        buf[n] = v;
        n += 1;
        if n == 4 {
            out.push((buf[0] << 2) | (buf[1] >> 4));
            out.push((buf[1] << 4) | (buf[2] >> 2));
            out.push((buf[2] << 6) | buf[3]);
            n = 0;
        }
    }
    if n == 2 {
        out.push((buf[0] << 2) | (buf[1] >> 4));
    } else if n == 3 {
        out.push((buf[0] << 2) | (buf[1] >> 4));
        out.push((buf[1] << 4) | (buf[2] >> 2));
    } else if n == 1 {
        return Err(GltfError::Message("base64 长度非法".into()));
    }
    Ok(out)
}

/// 缺省顶点色。
pub(crate) fn default_color() -> Color {
    Color::rgb(0.85, 0.85, 0.88)
}

pub(crate) fn aabb_from_positions(positions: &[[f32; 3]]) -> Aabb3 {
    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
    for p in positions {
        min.x = min.x.min(p[0]);
        min.y = min.y.min(p[1]);
        min.z = min.z.min(p[2]);
        max.x = max.x.max(p[0]);
        max.y = max.y.max(p[1]);
        max.z = max.z.max(p[2]);
    }
    if positions.is_empty() {
        Aabb3::from_min_max(Vec3::ZERO, Vec3::ZERO)
    } else {
        Aabb3::from_min_max(min, max)
    }
}
