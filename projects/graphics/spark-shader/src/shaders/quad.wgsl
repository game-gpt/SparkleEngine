struct Uniforms {
    screen: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var out: VsOut;
    let x = (v.pos.x / uniforms.screen.x) * 2.0 - 1.0;
    let y = 1.0 - (v.pos.y / uniforms.screen.y) * 2.0;
    out.clip = vec4<f32>(x, y, 0.0, 1.0);
    out.color = v.color;
    return out;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    return v.color;
}
