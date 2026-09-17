// Bloom 合成：scene + bloom * strength。

// 均匀缓冲布局须与 CPU `CompUniforms`（4×f32）一致。
// 勿用 `f32` + `vec3`：WGSL uniform 下 `vec3` 对齐 16，结构体会膨胀到 32。
struct CompUniforms {
    strength: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0)
var scene_tex: texture_2d<f32>;
@group(0) @binding(1)
var bloom_tex: texture_2d<f32>;
@group(0) @binding(2)
var samp: sampler;
@group(0) @binding(3)
var<uniform> comp: CompUniforms;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VsOut;
    let p = pos[vid];
    out.clip = vec4<f32>(p, 0.0, 1.0);
    out.uv = p * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_tex, samp, v.uv).rgb;
    let bloom = textureSample(bloom_tex, samp, v.uv).rgb;
    return vec4<f32>(scene + bloom * comp.strength, 1.0);
}
