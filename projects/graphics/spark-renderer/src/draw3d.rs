//! 3D 绘制列表（顶点色三角网格 + 可选 2D HUD）。

use std::sync::Arc;

use spark_core::Color;
use spark_geometry::Mat4;

use crate::draw::DrawList;

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
        if !vertices.is_empty() {
            self.meshes.push(MeshCmd { model, vertices });
        }
    }

    pub fn mesh_vec(&mut self, model: Mat4, vertices: Vec<MeshVertex>) {
        if !vertices.is_empty() {
            self.mesh(model, Arc::<[MeshVertex]>::from(vertices));
        }
    }
}
