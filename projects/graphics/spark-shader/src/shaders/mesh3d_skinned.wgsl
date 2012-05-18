// 不透明蒙皮网格：关节 palette 蒙皮 + 方向光 + 雾。
// group0: object（view_proj + model）
// group1: joints（64 × mat4）
// group2: lights

const MAX_JOINTS: u32 = 64;

struct ObjectUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
}

struct JointPalette {
    joints: array<mat4x4<f32>, 64>,
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
@group(1) @binding(0)
var<uniform> palette: JointPalette;
@group(2) @binding(0)
var<uniform> lights: FrameLights;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_n: vec3<f32>,
    @location(2) color: vec4<f32>,
}

fn skin_matrix(j: vec4<u32>, w: vec4<f32>) -> mat4x4<f32> {
    // 不变式：权重和 ≈ 1；关节下标 < MAX_JOINTS（越界钳到 0）。
    let j0 = min(j.x, MAX_JOINTS - 1u);
    let j1 = min(j.y, MAX_JOINTS - 1u);
    let j2 = min(j.z, MAX_JOINTS - 1u);
    let j3 = min(j.w, MAX_JOINTS - 1u);
    return palette.joints[j0] * w.x
        + palette.joints[j1] * w.y
        + palette.joints[j2] * w.z
        + palette.joints[j3] * w.w;
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    let skin = skin_matrix(v.joints, v.weights);
    let local = skin * vec4<f32>(v.pos, 1.0);
    let world = object.model * local;
    out.clip = object.view_proj * world;
    out.world_pos = world.xyz;
    let n4 = object.model * (skin * vec4<f32>(v.normal, 0.0));
    let n = n4.xyz;
    let nlen = length(n);
    out.world_n = select(vec3<f32>(0.0, 1.0, 0.0), n / nlen, nlen > 1e-5);
    out.color = v.color;
    let _uv = v.uv;
    return out;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(v.world_n);
    let sun = normalize(lights.sun_dir.xyz);
    let ndotl = max(dot(n, sun), 0.0);
    let lit = lights.ambient.xyz + lights.sun_color.xyz * ndotl;
    var rgb = v.color.xyz * lit;
    let dist = length(v.world_pos - lights.eye.xyz);
    let fog_t = 1.0 - exp(-lights.fog_color_density.w * dist);
    rgb = mix(rgb, lights.fog_color_density.xyz, clamp(fog_t, 0.0, 1.0));
    // 与 LitSolidMesh3d 一致的简易曝光 + ACES。
    rgb = aces_tonemap(rgb * 1.15);
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
