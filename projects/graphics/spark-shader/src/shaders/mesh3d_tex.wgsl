// 不透明纹理网格：方向光 + 环境光 + 指数距离雾。

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
    @location(1) world_n: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    let world = object.model * vec4<f32>(v.pos, 1.0);
    out.clip = object.view_proj * world;
    out.world_pos = world.xyz;
    let n = (object.model * vec4<f32>(v.normal, 0.0)).xyz;
    let nlen = length(n);
    out.world_n = select(vec3<f32>(0.0, 1.0, 0.0), n / nlen, nlen > 1e-5);
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo_tex, albedo_samp, v.uv);
    let n = normalize(v.world_n);
    let sun = normalize(lights.sun_dir.xyz);
    let ndotl = max(dot(n, sun), 0.0);
    let lit = lights.ambient.xyz + lights.sun_color.xyz * ndotl;
    var rgb = texel.xyz * v.color.xyz * lit;
    let dist = length(v.world_pos - lights.eye.xyz);
    let fog_t = 1.0 - exp(-lights.fog_color_density.w * dist);
    rgb = mix(rgb, lights.fog_color_density.xyz, clamp(fog_t, 0.0, 1.0));
    rgb = aces_tonemap(rgb * 1.15);
    return vec4<f32>(rgb, texel.w * v.color.w);
}

fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}
