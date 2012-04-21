//! 3D 绘制列表（顶点色三角网格 + 可选 2D HUD + 驻留键 / 裁剪）。

use std::sync::Arc;

use spark_core::Color;
use spark_geometry::{Aabb3, Mat4, Vec3};

use crate::draw::DrawList;
use crate::frustum::{CullParams, Frustum};

/// 驻留网格标识（跨帧稳定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshId(pub u64);

/// GPU 驻留键：`id` 稳定，`revision` 在 CPU 顶点变更时递增。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshResidentKey {
    pub id: MeshId,
    pub revision: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MeshVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

impl MeshVertex {
    pub fn new(x: f32, y: f32, z: f32, color: Color) -> Self {
        Self {
            pos: [x, y, z],
            color: color.to_array(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshCmd {
    pub model: Mat4,
    /// 共享顶点，避免每帧深拷贝巨型网格。
    pub vertices: Arc<[MeshVertex]>,
    /// `Some` 时后端可按键缓存 GPU 缓冲；`revision` 变化则重新上传。
    pub resident: Option<MeshResidentKey>,
    /// 局部空间 AABB。提供后参与视锥 / 距离裁剪。
    pub local_aabb: Option<Aabb3>,
}

impl MeshCmd {
    pub fn world_aabb(&self) -> Option<Aabb3> {
        self.local_aabb.map(|a| a.transformed(self.model))
    }
}

/// 一帧 3D 绘制 + HUD。
#[derive(Debug)]
pub struct DrawList3d {
    pub clear: Color,
    pub view_proj: Mat4,
    pub meshes: Vec<MeshCmd>,
    pub hud: DrawList,
}

impl DrawList3d {
    pub fn new(clear: Color, view_proj: Mat4) -> Self {
        Self {
            clear,
            view_proj,
            meshes: Vec::new(),
            hud: DrawList::new(Color::rgba(0.0, 0.0, 0.0, 0.0)),
        }
    }

    pub fn mesh(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>) {
        self.push_mesh(model, vertices, None, None);
    }

    pub fn mesh_vec(&mut self, model: Mat4, vertices: Vec<MeshVertex>) {
        if !vertices.is_empty() {
            self.mesh(model, Arc::<[MeshVertex]>::from(vertices));
        }
    }

    /// 带局部 AABB 的瞬时网格（可裁剪，每帧上传）。
    pub fn mesh_culled(&mut self, model: Mat4, vertices: Arc<[MeshVertex]>, local_aabb: Aabb3) {
        self.push_mesh(model, vertices, None, Some(local_aabb));
    }

    /// GPU 驻留网格：后端按 `key` 缓存 VBO。
    pub fn mesh_resident(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        key: MeshResidentKey,
        local_aabb: Option<Aabb3>,
    ) {
        self.push_mesh(model, vertices, Some(key), local_aabb);
    }

    fn push_mesh(
        &mut self,
        model: Mat4,
        vertices: Arc<[MeshVertex]>,
        resident: Option<MeshResidentKey>,
        local_aabb: Option<Aabb3>,
    ) {
        if vertices.is_empty() {
            return;
        }
        self.meshes.push(MeshCmd {
            model,
            vertices,
            resident,
            local_aabb,
        });
    }

    /// 按视锥与可选距离剔除无 AABB 或不可见网格。无 AABB 的条目始终保留。
    pub fn retain_visible(&mut self, cull: CullParams) {
        let frustum = Frustum::from_view_proj(&self.view_proj);
        self.meshes.retain(|m| {
            let Some(world) = m.world_aabb() else {
                return true;
            };
            if let Some(max_d) = cull.max_distance {
                let c = world.center();
                let dist = (c - cull.eye).length() - world.extents().length();
                if dist > max_d {
                    return false;
                }
            }
            frustum.intersects_aabb(&world)
        });
    }

    /// 仅距离剔除（不建视锥），无 AABB 保留。
    pub fn retain_within_distance(&mut self, eye: Vec3, max_distance: f32) {
        let max_d = max_distance.max(0.0);
        self.meshes.retain(|m| {
            let Some(world) = m.world_aabb() else {
                return true;
            };
            let c = world.center();
            let dist = (c - eye).length() - world.extents().length();
            dist <= max_d
        });
    }
}
