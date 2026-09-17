// 世界自发光纹理网格：无方向光，仅纹理×顶点色 + 轻雾；供 additive Emissive pass。

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

@group(0) @binding(0)
var<uniform> object: ObjectUniforms;
@group(0) @binding(1)
var albedo_tex: texture_2d<f32>;
@group(0) @binding(2)
var albedo_samp: sampler;
@group(1) @binding(0)
var<uniform> lights: FrameLights;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    let world = object.model * vec4<f32>(v.pos, 1.0);
    out.clip = object.view_proj * world;
    out.world_pos = world.xyz;
    out.uv = v.uv;
    out.color = v.color;
    let _n = v.normal;
    return out;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo_tex, albedo_samp, v.uv);
    var rgb = texel.xyz * v.color.xyz;
    let dist = length(v.world_pos - lights.eye.xyz);
    // 发光体少雾一点，远景仍能看见引擎/岩浆点缀。
    let fog_t = 1.0 - exp(-lights.fog_color_density.w * dist * 0.45);
    rgb = mix(rgb, lights.fog_color_density.xyz, clamp(fog_t, 0.0, 1.0) * 0.35);
    return vec4<f32>(rgb, texel.w * v.color.w);
}
