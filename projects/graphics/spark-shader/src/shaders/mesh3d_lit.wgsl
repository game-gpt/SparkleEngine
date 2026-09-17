// 不透明顶点色网格：方向光 + 环境光 + 指数距离雾 + 级联太阳阴影。
// SkyPass 继续使用 mesh3d.wgsl（无光照）。

struct ObjectUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
}

struct FrameLights {
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    fog_color_density: vec4<f32>,
    eye: vec4<f32>,
}

struct ShadowUniforms {
    light_view_proj: array<mat4x4<f32>, 3>,
    // x=enabled y=bias z=strength w=cascade_count
    params: vec4<f32>,
    // xyz=split_end[0..2] w=texel
    splits: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> object: ObjectUniforms;
@group(1) @binding(0)
var<uniform> lights: FrameLights;
@group(2) @binding(0)
var shadow_map: texture_depth_2d_array;
@group(2) @binding(1)
var shadow_samp: sampler_comparison;
@group(2) @binding(2)
var<uniform> shadow: ShadowUniforms;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_n: vec3<f32>,
    @location(2) color: vec4<f32>,
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    let world = object.model * vec4<f32>(v.pos, 1.0);
    out.clip = object.view_proj * world;
    out.world_pos = world.xyz;
    // 均匀缩放 / 刚体 model：w=0 变换法线即可。
    let n = (object.model * vec4<f32>(v.normal, 0.0)).xyz;
    let nlen = length(n);
    out.world_n = select(vec3<f32>(0.0, 1.0, 0.0), n / nlen, nlen > 1e-5);
    out.color = v.color;
    return out;
}

fn pick_cascade(dist: f32) -> i32 {
    let count = i32(shadow.params.w);
    if count <= 1 {
        return 0;
    }
    if dist < shadow.splits.x {
        return 0;
    }
    if count >= 2 && dist < shadow.splits.y {
        return 1;
    }
    if count >= 3 {
        return 2;
    }
    return max(count - 1, 0);
}

fn sun_shadow(world_pos: vec3<f32>) -> f32 {
    if shadow.params.x < 0.5 {
        return 1.0;
    }
    let dist = length(world_pos - lights.eye.xyz);
    let count = i32(shadow.params.w);
    // 超出最远级联覆盖半径：远景 LOD 像素跳过矩阵与深度采样。
    let cover = select(
        shadow.splits.x,
        select(shadow.splits.y, shadow.splits.z, count >= 3),
        count >= 2,
    );
    if dist > cover {
        return 1.0;
    }
    let layer = pick_cascade(dist);
    let lp = shadow.light_view_proj[layer] * vec4<f32>(world_pos, 1.0);
    let ndc = lp.xyz / max(lp.w, 1e-6);
    // wgpu：裁剪 Y 向上，深度纹理 (0,0) 在左上 → 采样时翻转 Y。
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);
    let depth = ndc.z;
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || depth < 0.0 || depth > 1.0 {
        return 1.0;
    }
    // 单点比较：体素远景满屏时 3×3 PCF 成本过高。
    let bias = shadow.params.y;
    let lit = textureSampleCompare(shadow_map, shadow_samp, uv, layer, depth - bias);
    return mix(1.0 - shadow.params.z, 1.0, lit);
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(v.world_n);
    let sun = normalize(lights.sun_dir.xyz);
    let ndotl = max(dot(n, sun), 0.0);
    let sh = sun_shadow(v.world_pos);
    let lit = lights.ambient.xyz + lights.sun_color.xyz * ndotl * sh;
    var rgb = v.color.xyz * lit;
    let dist = length(v.world_pos - lights.eye.xyz);
    let fog_t = 1.0 - exp(-lights.fog_color_density.w * dist);
    rgb = mix(rgb, lights.fog_color_density.xyz, clamp(fog_t, 0.0, 1.0));
    // `eye.w` = 曝光倍率（`FrameLights3d.exposure`）。
    let exposure = select(1.15, lights.eye.w, lights.eye.w > 0.01);
    rgb = aces_tonemap(rgb * exposure);
    return vec4<f32>(rgb, v.color.w);
}

fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}
