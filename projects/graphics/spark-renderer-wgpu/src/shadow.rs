//! 太阳正交阴影图（支持 1..3 级联深度数组）。

use bytemuck::{Pod, Zeroable};
use spark_renderer::{
    DrawList3d, Frustum, MAX_SHADOW_CASCADES, MeshCmd, MeshResidentKey, MeshVertex, ShadowParams3d, TexMeshCmd, TexMeshVertex,
};
use spark_shader::{BuiltinShader, create_builtin};
use wgpu::util::DeviceExt;

use crate::game3d::mat4_to_cols_pub;

// 近场单级联够用；2048² 深度填充在转视角时过重。
const MAP_SIZE: u32 = 1024;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ObjectUniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ShadowUniformsGpu {
    pub light_view_proj: [[[f32; 4]; 4]; MAX_SHADOW_CASCADES],
    /// x=enabled y=bias z=strength w=cascade_count
    pub params: [f32; 4],
    /// xyz = split_end[0..3]；w = 1/MAP_SIZE（PCF 纹素）
    pub splits: [f32; 4],
}

struct DepthResident {
    buffer: wgpu::Buffer,
    vertex_count: u32,
    revision: u32,
}

/// 阴影图 + 深度管线 + 比较采样绑定。
pub struct ShadowMapGpu {
    /// 保持纹理存活；采样通过 `sample_view`。
    _map: wgpu::Texture,
    _sample_view: wgpu::TextureView,
    layer_views: [wgpu::TextureView; MAX_SHADOW_CASCADES],
    depth_pipeline_mesh: wgpu::RenderPipeline,
    depth_pipeline_tex: wgpu::RenderPipeline,
    object_uniform: wgpu::Buffer,
    object_uniform_stride: u64,
    object_uniform_slots: usize,
    object_bind: wgpu::BindGroup,
    shadow_bgl: wgpu::BindGroupLayout,
    shadow_uniform: wgpu::Buffer,
    /// 比较采样器；绑定组持有引用，字段保活。
    _shadow_samp: wgpu::Sampler,
    shadow_bind: wgpu::BindGroup,
    mesh_cache: std::collections::HashMap<u64, DepthResident>,
    tex_cache: std::collections::HashMap<u64, DepthResident>,
    transient_mesh: wgpu::Buffer,
    transient_tex: wgpu::Buffer,
    transient_cap: u64,
}

impl ShadowMapGpu {
    pub fn new(device: &wgpu::Device) -> Self {
        let map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sun-shadow-map"),
            size: wgpu::Extent3d { width: MAP_SIZE, height: MAP_SIZE, depth_or_array_layers: MAX_SHADOW_CASCADES as u32 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let sample_view = map.create_view(&wgpu::TextureViewDescriptor {
            label: Some("sun-shadow-sample"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let layer_views = std::array::from_fn(|i| {
            map.create_view(&wgpu::TextureViewDescriptor {
                label: Some("sun-shadow-layer"),
                format: None,
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: i as u32,
                array_layer_count: Some(1),
                usage: None,
            })
        });

        let depth_shader = create_builtin(device, BuiltinShader::DepthOnlyMesh3d);
        let object_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow-object-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: crate::dyn_ubo::binding_size(std::mem::size_of::<ObjectUniforms>() as u64),
                },
                count: None,
            }],
        });
        let object_uniform_stride = crate::dyn_ubo::uniform_stride(device, std::mem::size_of::<ObjectUniforms>() as u64);
        let object_uniform_slots = 512usize;
        let object_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow-object-uniform-ring"),
            size: object_uniform_stride * object_uniform_slots as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let object_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow-object-bg"),
            layout: &object_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &object_uniform,
                    offset: 0,
                    size: crate::dyn_ubo::binding_size(std::mem::size_of::<ObjectUniforms>() as u64),
                }),
            }],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow-depth-pl"),
            bind_group_layouts: &[Some(&object_bgl)],
            immediate_size: 0,
        });

        let depth_pipeline_mesh = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow-depth-mesh"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &depth_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    // MeshVertGpu: pos+normal+color，只读 pos。
                    array_stride: (3 + 3 + 4) * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                })],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 1.5, clamp: 0.0 },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let depth_pipeline_tex = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow-depth-tex"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &depth_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    // TexVertGpu: pos+normal+uv+color
                    array_stride: (3 + 3 + 2 + 4) * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                })],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 1.5, clamp: 0.0 },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let shadow_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow-sample-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let shadow_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow-uniform"),
            size: std::mem::size_of::<ShadowUniformsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_samp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow-compare"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shadow_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow-sample-bg"),
            layout: &shadow_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&sample_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&shadow_samp) },
                wgpu::BindGroupEntry { binding: 2, resource: shadow_uniform.as_entire_binding() },
            ],
        });

        let transient_cap = 256_000u64;
        let transient_mesh = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow-transient-mesh"),
            size: transient_cap * (10 * 4),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let transient_tex = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow-transient-tex"),
            size: transient_cap * (12 * 4),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            _map: map,
            _sample_view: sample_view,
            layer_views,
            depth_pipeline_mesh,
            depth_pipeline_tex,
            object_uniform,
            object_uniform_stride,
            object_uniform_slots,
            object_bind,
            shadow_bgl,
            shadow_uniform,
            _shadow_samp: shadow_samp,
            shadow_bind,
            mesh_cache: Default::default(),
            tex_cache: Default::default(),
            transient_mesh,
            transient_tex,
            transient_cap,
        }
    }

    pub fn sample_bgl(&self) -> &wgpu::BindGroupLayout {
        &self.shadow_bgl
    }

    pub fn sample_bind(&self) -> &wgpu::BindGroup {
        &self.shadow_bind
    }

    pub fn write_params(&self, queue: &wgpu::Queue, list: &DrawList3d) {
        let s = &list.shadow;
        let gpu = pack_shadow_uniforms(s);
        queue.write_buffer(&self.shadow_uniform, 0, bytemuck::bytes_of(&gpu));
    }

    /// 将不透明投射体写入各级联阴影层。
    pub fn render_casters(&mut self, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device, queue: &wgpu::Queue, list: &DrawList3d) {
        if !list.shadow.enabled {
            return;
        }
        self.prepare_residents(device, list);

        let count = list.shadow.cascade_count.clamp(1, MAX_SHADOW_CASCADES as u32) as usize;
        for layer in 0..count {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sun-shadow-cascade"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.layer_views[layer],
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            let light_vp = list.shadow.light_view_proj[layer];
            let light_frustum = Frustum::from_view_proj(&light_vp);
            let mut ubo_slot = 0usize;
            pass.set_pipeline(&self.depth_pipeline_mesh);
            self.draw_mesh_depth(&mut pass, queue, &list.meshes, &light_vp, &light_frustum, &mut ubo_slot);

            pass.set_pipeline(&self.depth_pipeline_tex);
            self.draw_tex_depth(&mut pass, queue, &list.tex_meshes, &light_vp, &light_frustum, &mut ubo_slot);
        }
    }

    fn prepare_residents(&mut self, device: &wgpu::Device, list: &DrawList3d) {
        let mut uploads = 0usize;
        for m in &list.meshes {
            if let Some(key) = m.resident {
                if Self::mesh_fresh(self.mesh_cache.get(&key.id.0), key, m.vertices.len()) {
                    continue;
                }
                if uploads >= crate::RESIDENT_UPLOADS_PER_FRAME {
                    continue;
                }
                self.ensure_mesh(device, key.id.0, key.revision, &m.vertices);
                uploads += 1;
            }
        }
        for m in &list.tex_meshes {
            if let Some(key) = m.resident {
                if Self::tex_fresh(self.tex_cache.get(&key.id.0), key, m.vertices.len()) {
                    continue;
                }
                if uploads >= crate::RESIDENT_UPLOADS_PER_FRAME {
                    continue;
                }
                self.ensure_tex(device, key.id.0, key.revision, &m.vertices);
                uploads += 1;
            }
        }
    }

    fn mesh_fresh(entry: Option<&DepthResident>, key: MeshResidentKey, n: usize) -> bool {
        match entry {
            Some(e) => e.revision == key.revision && e.vertex_count as usize == n,
            None => n == 0,
        }
    }

    fn tex_fresh(entry: Option<&DepthResident>, key: MeshResidentKey, n: usize) -> bool {
        match entry {
            Some(e) => e.revision == key.revision && e.vertex_count as usize == n,
            None => n == 0,
        }
    }

    fn ensure_mesh(&mut self, device: &wgpu::Device, id: u64, revision: u32, verts: &[MeshVertex]) {
        if let Some(e) = self.mesh_cache.get(&id) {
            if e.revision == revision && e.vertex_count as usize == verts.len() {
                return;
            }
        }
        if verts.is_empty() {
            self.mesh_cache.remove(&id);
            return;
        }
        // 上传完整 MeshVert 布局（与主缓存一致），深度管线只读 pos。
        let mut packed = Vec::with_capacity(verts.len() * 10);
        for v in verts {
            packed.extend_from_slice(&v.pos);
            packed.extend_from_slice(&v.normal);
            packed.extend_from_slice(&v.color);
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow-mesh-resident"),
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.mesh_cache.insert(id, DepthResident { buffer, vertex_count: verts.len() as u32, revision });
    }

    fn ensure_tex(&mut self, device: &wgpu::Device, id: u64, revision: u32, verts: &[TexMeshVertex]) {
        if let Some(e) = self.tex_cache.get(&id) {
            if e.revision == revision && e.vertex_count as usize == verts.len() {
                return;
            }
        }
        if verts.is_empty() {
            self.tex_cache.remove(&id);
            return;
        }
        let mut packed = Vec::with_capacity(verts.len() * 12);
        for v in verts {
            packed.extend_from_slice(&v.pos);
            packed.extend_from_slice(&v.normal);
            packed.extend_from_slice(&v.uv);
            packed.extend_from_slice(&v.color);
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow-tex-resident"),
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.tex_cache.insert(id, DepthResident { buffer, vertex_count: verts.len() as u32, revision });
    }

    fn draw_mesh_depth(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        queue: &wgpu::Queue,
        meshes: &[MeshCmd],
        light_vp: &spark_geometry::Mat4,
        light_frustum: &Frustum,
        ubo_slot: &mut usize,
    ) {
        let vp = mat4_to_cols_pub(light_vp);
        let mut planned: Vec<(u32, usize)> = Vec::with_capacity(meshes.len());
        for (mi, mesh) in meshes.iter().enumerate() {
            if !mesh.casts_shadow {
                continue;
            }
            if mesh.world_aabb().is_some_and(|aabb| !light_frustum.intersects_aabb(&aabb)) {
                continue;
            }
            let drawable = if let Some(key) = mesh.resident {
                self.mesh_cache.contains_key(&key.id.0)
            }
            else {
                !mesh.vertices.is_empty() && (mesh.vertices.len() as u64) <= self.transient_cap
            };
            if !drawable {
                continue;
            }
            if *ubo_slot >= self.object_uniform_slots {
                break;
            }
            let uniforms = ObjectUniforms { view_proj: vp, model: mat4_to_cols_pub(&mesh.model) };
            let off = *ubo_slot as u64 * self.object_uniform_stride;
            queue.write_buffer(&self.object_uniform, off, bytemuck::bytes_of(&uniforms));
            planned.push((off as u32, mi));
            *ubo_slot += 1;
        }
        for (dyn_off, mi) in planned {
            let mesh = &meshes[mi];
            pass.set_bind_group(0, &self.object_bind, &[dyn_off]);
            if let Some(key) = mesh.resident {
                let Some(e) = self.mesh_cache.get(&key.id.0)
                else {
                    continue;
                };
                pass.set_vertex_buffer(0, e.buffer.slice(..));
                pass.draw(0..e.vertex_count, 0..1);
            }
            else if (mesh.vertices.len() as u64) <= self.transient_cap {
                let mut packed = Vec::with_capacity(mesh.vertices.len() * 10);
                for v in mesh.vertices.iter() {
                    packed.extend_from_slice(&v.pos);
                    packed.extend_from_slice(&v.normal);
                    packed.extend_from_slice(&v.color);
                }
                queue.write_buffer(&self.transient_mesh, 0, bytemuck::cast_slice(&packed));
                pass.set_vertex_buffer(0, self.transient_mesh.slice(..));
                pass.draw(0..mesh.vertices.len() as u32, 0..1);
            }
        }
    }

    fn draw_tex_depth(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        queue: &wgpu::Queue,
        meshes: &[TexMeshCmd],
        light_vp: &spark_geometry::Mat4,
        light_frustum: &Frustum,
        ubo_slot: &mut usize,
    ) {
        let vp = mat4_to_cols_pub(light_vp);
        let mut planned: Vec<(u32, usize)> = Vec::with_capacity(meshes.len());
        for (mi, mesh) in meshes.iter().enumerate() {
            if !mesh.casts_shadow {
                continue;
            }
            if mesh.world_aabb().is_some_and(|aabb| !light_frustum.intersects_aabb(&aabb)) {
                continue;
            }
            let drawable = if let Some(key) = mesh.resident {
                self.tex_cache.contains_key(&key.id.0)
            }
            else {
                !mesh.vertices.is_empty() && (mesh.vertices.len() as u64) <= self.transient_cap
            };
            if !drawable {
                continue;
            }
            if *ubo_slot >= self.object_uniform_slots {
                break;
            }
            let uniforms = ObjectUniforms { view_proj: vp, model: mat4_to_cols_pub(&mesh.model) };
            let off = *ubo_slot as u64 * self.object_uniform_stride;
            queue.write_buffer(&self.object_uniform, off, bytemuck::bytes_of(&uniforms));
            planned.push((off as u32, mi));
            *ubo_slot += 1;
        }
        for (dyn_off, mi) in planned {
            let mesh = &meshes[mi];
            pass.set_bind_group(0, &self.object_bind, &[dyn_off]);
            if let Some(key) = mesh.resident {
                let Some(e) = self.tex_cache.get(&key.id.0)
                else {
                    continue;
                };
                pass.set_vertex_buffer(0, e.buffer.slice(..));
                pass.draw(0..e.vertex_count, 0..1);
            }
            else if (mesh.vertices.len() as u64) <= self.transient_cap {
                let mut packed = Vec::with_capacity(mesh.vertices.len() * 12);
                for v in mesh.vertices.iter() {
                    packed.extend_from_slice(&v.pos);
                    packed.extend_from_slice(&v.normal);
                    packed.extend_from_slice(&v.uv);
                    packed.extend_from_slice(&v.color);
                }
                queue.write_buffer(&self.transient_tex, 0, bytemuck::cast_slice(&packed));
                pass.set_vertex_buffer(0, self.transient_tex.slice(..));
                pass.draw(0..mesh.vertices.len() as u32, 0..1);
            }
        }
    }
}

fn pack_shadow_uniforms(s: &ShadowParams3d) -> ShadowUniformsGpu {
    let mut light_view_proj = [[[0.0f32; 4]; 4]; MAX_SHADOW_CASCADES];
    for i in 0..MAX_SHADOW_CASCADES {
        light_view_proj[i] = mat4_to_cols_pub(&s.light_view_proj[i]);
    }
    let count = s.cascade_count.clamp(1, MAX_SHADOW_CASCADES as u32) as f32;
    ShadowUniformsGpu {
        light_view_proj,
        params: [if s.enabled { 1.0 } else { 0.0 }, s.bias, s.strength, count],
        splits: [s.split_end[0], s.split_end[1], s.split_end[2], 1.0 / MAP_SIZE as f32],
    }
}
