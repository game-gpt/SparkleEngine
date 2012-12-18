//! 3D 蒙皮网格 GPU：关节 palette + `SkinnedMeshCmd`。
//!
//! 首切仅顶点色蒙皮（`texture` 字段预留，暂不采样）。

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use spark_core::SparkError;
use spark_geometry::Mat4;
use spark_renderer::{DrawList3d, MAX_SKIN_JOINTS, MeshResidentKey, SkinnedMeshCmd, SkinnedVertex};
use spark_shader::{BuiltinShader, create_builtin};
use wgpu::util::DeviceExt;

use crate::game3d::mat4_to_cols_pub;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SkinnedVertGpu {
    pos: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    joints: [u32; 4],
    weights: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms3d {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct JointPaletteGpu {
    joints: [[[f32; 4]; 4]; MAX_SKIN_JOINTS],
}

struct ResidentSkinned {
    buffer: wgpu::Buffer,
    vertex_count: u32,
    revision: u32,
}

/// 蒙皮网格管线与缓存。
pub struct SkinnedMeshGpu {
    pipeline: wgpu::RenderPipeline,
    object_bind: wgpu::BindGroup,
    object_uniform: wgpu::Buffer,
    joints_bind: wgpu::BindGroup,
    joints_uniform: wgpu::Buffer,
    transient_vbo: wgpu::Buffer,
    transient_cap: u64,
    mesh_cache: HashMap<u64, ResidentSkinned>,
}

impl SkinnedMeshGpu {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, lights_bgl: &wgpu::BindGroupLayout) -> Self {
        let shader = create_builtin(device, BuiltinShader::SkinnedMesh3d);
        let object_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skinned-object-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let joints_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skinned-joints-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let object_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skinned-object-uniform"),
            size: std::mem::size_of::<Uniforms3d>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let joints_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skinned-joints-uniform"),
            size: std::mem::size_of::<JointPaletteGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let object_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skinned-object-bg"),
            layout: &object_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: object_uniform.as_entire_binding() }],
        });
        let joints_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skinned-joints-bg"),
            layout: &joints_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: joints_uniform.as_entire_binding() }],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skinned-mesh3d-pl"),
            bind_group_layouts: &[Some(&object_bgl), Some(&joints_bgl), Some(lights_bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skinned-mesh3d"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SkinnedVertGpu>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                        3 => Float32x4,
                        4 => Uint32x4,
                        5 => Float32x4
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let transient_cap = 65_536u64;
        let transient_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skinned-transient-vbo"),
            size: transient_cap * std::mem::size_of::<SkinnedVertGpu>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { pipeline, object_bind, object_uniform, joints_bind, joints_uniform, transient_vbo, transient_cap, mesh_cache: HashMap::new() }
    }

    pub fn prepare_residents(&mut self, device: &wgpu::Device, list: &DrawList3d) {
        let mut uploads = 0usize;
        for cmd in &list.skinned_meshes {
            if let Some(key) = cmd.resident {
                if let Some(entry) = self.mesh_cache.get(&key.id.0) {
                    if entry.revision == key.revision && entry.vertex_count as usize == cmd.vertices.len() {
                        continue;
                    }
                }
                else if cmd.vertices.is_empty() {
                    continue;
                }
                if uploads >= crate::RESIDENT_UPLOADS_PER_FRAME {
                    continue;
                }
                self.ensure_resident(device, key, &cmd.vertices);
                uploads += 1;
            }
        }
    }

    fn ensure_resident(&mut self, device: &wgpu::Device, key: MeshResidentKey, vertices: &[SkinnedVertex]) {
        if let Some(entry) = self.mesh_cache.get(&key.id.0) {
            if entry.revision == key.revision && entry.vertex_count as usize == vertices.len() {
                return;
            }
        }
        let gpu = pack_verts(vertices);
        if gpu.is_empty() {
            self.mesh_cache.remove(&key.id.0);
            return;
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("resident-skinned-vbo"),
            contents: bytemuck::cast_slice(&gpu),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.mesh_cache.insert(key.id.0, ResidentSkinned { buffer, vertex_count: gpu.len() as u32, revision: key.revision });
    }

    fn ensure_transient_cap(&mut self, device: &wgpu::Device, need: u64) -> Result<(), SparkError> {
        if need <= self.transient_cap {
            return Ok(());
        }
        let mut cap = self.transient_cap.max(4096);
        while cap < need {
            cap = cap.saturating_mul(2);
        }
        self.transient_vbo = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("skinned-transient-vbo"),
            size: cap * std::mem::size_of::<SkinnedVertGpu>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.transient_cap = cap;
        Ok(())
    }

    pub fn draw(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        list: &DrawList3d,
        lights_bind: &wgpu::BindGroup,
        view_proj: &Mat4,
    ) -> Result<(), SparkError> {
        if list.skinned_meshes.is_empty() {
            return Ok(());
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.object_bind, &[]);
        pass.set_bind_group(1, &self.joints_bind, &[]);
        pass.set_bind_group(2, lights_bind, &[]);

        for cmd in &list.skinned_meshes {
            self.draw_one(pass, device, queue, cmd, view_proj)?;
        }
        Ok(())
    }

    fn draw_one(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cmd: &SkinnedMeshCmd,
        view_proj: &Mat4,
    ) -> Result<(), SparkError> {
        let uniforms = Uniforms3d { view_proj: mat4_to_cols_pub(view_proj), model: mat4_to_cols_pub(&cmd.model) };
        queue.write_buffer(&self.object_uniform, 0, bytemuck::bytes_of(&uniforms));
        queue.write_buffer(&self.joints_uniform, 0, bytemuck::bytes_of(&pack_palette(&cmd.joint_palette)));

        if let Some(key) = cmd.resident {
            if let Some(entry) = self.mesh_cache.get(&key.id.0) {
                if entry.revision == key.revision && entry.vertex_count > 0 {
                    pass.set_vertex_buffer(0, entry.buffer.slice(..));
                    pass.draw(0..entry.vertex_count, 0..1);
                    return Ok(());
                }
            }
        }

        let gpu = pack_verts(&cmd.vertices);
        if gpu.is_empty() {
            return Ok(());
        }
        self.ensure_transient_cap(device, gpu.len() as u64)?;
        queue.write_buffer(&self.transient_vbo, 0, bytemuck::cast_slice(&gpu));
        pass.set_vertex_buffer(0, self.transient_vbo.slice(..));
        pass.draw(0..gpu.len() as u32, 0..1);
        Ok(())
    }
}

fn pack_verts(vertices: &[SkinnedVertex]) -> Vec<SkinnedVertGpu> {
    vertices
        .iter()
        .map(|v| SkinnedVertGpu { pos: v.pos, normal: v.normal, uv: v.uv, color: v.color, joints: v.joints, weights: v.weights })
        .collect()
}

fn pack_palette(palette: &[Mat4]) -> JointPaletteGpu {
    let mut joints = [[[0.0f32; 4]; 4]; MAX_SKIN_JOINTS];
    // 缺省为单位阵，避免未绑定关节把顶点拉到原点。
    for slot in joints.iter_mut() {
        *slot = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    }
    let n = palette.len().min(MAX_SKIN_JOINTS);
    for i in 0..n {
        joints[i] = mat4_to_cols_pub(&palette[i]);
    }
    JointPaletteGpu { joints }
}
