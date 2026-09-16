struct Uniforms {
    screen: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;
@group(0) @binding(1)
var atlas_tex: texture_2d<f32>;
@group(0) @binding(2)
var atlas_samp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    let x = (v.pos.x / uniforms.screen.x) * 2.0 - 1.0;
    let y = 1.0 - (v.pos.y / uniforms.screen.y) * 2.0;
    out.clip = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(atlas_tex, atlas_samp, v.uv).r;
    return vec4<f32>(v.color.rgb, v.color.a * a);
}
