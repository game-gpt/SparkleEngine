// 视线方向大气穹顶（SkyPass）。世界原点为眼点时，`normalize(world_pos)` 即视线。
// 顶点色作美术 tint；散射主项由 `FrameLights.sun_dir` / 太阳色驱动。

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
@group(1) @binding(0)
var<uniform> lights: FrameLights;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) tint: vec4<f32>,
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    let world = object.model * vec4<f32>(v.pos, 1.0);
    out.clip = object.view_proj * world;
    out.world_pos = world.xyz;
    out.tint = v.color;
    let _n = v.normal;
    return out;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let view = normalize(v.world_pos);
    let sun = normalize(lights.sun_dir.xyz);
    let elev = view.y;
    let cos_sun = clamp(dot(view, sun), -1.0, 1.0);

    // 天顶蓝紫、地平暖金、天底暗；随太阳高度略调。
    let sun_h = clamp(sun.y, 0.0, 1.0);
    let zenith = mix(vec3<f32>(0.10, 0.08, 0.32), vec3<f32>(0.18, 0.28, 0.62), sun_h);
    let horizon = mix(vec3<f32>(0.95, 0.48, 0.22), vec3<f32>(0.72, 0.82, 0.95), sun_h);
    let nadir = vec3<f32>(0.04, 0.035, 0.06);

    var base: vec3<f32>;
    if elev >= 0.0 {
        let t = pow(clamp(elev, 0.0, 1.0), 0.65);
        base = mix(horizon, zenith, t);
    } else {
        let t = pow(clamp(-elev, 0.0, 1.0), 0.7);
        base = mix(horizon, nadir, t);
    }

    // Rayleigh 近似：背光略偏蓝。
    let rayleigh = pow(1.0 - abs(cos_sun) * 0.5, 2.0) * 0.12;
    base += vec3<f32>(0.05, 0.09, 0.18) * rayleigh;

    // Mie：太阳附近暖辉（配合 corona mesh，此处偏软）。
    let mie = pow(max(cos_sun, 0.0), 24.0);
    let mie2 = pow(max(cos_sun, 0.0), 8.0);
    base += lights.sun_color.xyz * mie * 0.85;
    base += mix(horizon, lights.sun_color.xyz, 0.5) * mie2 * 0.22;

    // 地平雾带。
    let haze = pow(clamp(1.0 - abs(elev) * 1.9, 0.0, 1.0), 1.15);
    base = mix(base, horizon * 1.05, haze * 0.45);

    var rgb = base * v.tint.xyz;
    rgb = aces_tonemap(rgb * 1.05);
    return vec4<f32>(rgb, v.tint.w);
}

fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}
