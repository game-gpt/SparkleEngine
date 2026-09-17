// Bloom：全屏三角。提取 / 模糊共用 src。

struct BlurUniforms {
    direction: vec2<f32>,
    strength: f32,
    _pad: f32,
}

@group(0) @binding(0)
var src_tex: texture_2d<f32>;
@group(0) @binding(1)
var src_samp: sampler;
@group(0) @binding(2)
var<uniform> blur: BlurUniforms;

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
fn fs_extract(v: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(src_tex, src_samp, v.uv).rgb;
    let luma = max(max(c.r, c.g), c.b);
    let t = smoothstep(0.78, 1.15, luma);
    return vec4<f32>(c * t, 1.0);
}

@fragment
fn fs_blur(v: VsOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(src_tex, 0));
    let dir = blur.direction * texel;
    let w0 = 0.227027;
    let w1 = 0.316216;
    let w2 = 0.070270;
    var rgb = textureSample(src_tex, src_samp, v.uv).rgb * w0;
    rgb += textureSample(src_tex, src_samp, v.uv + dir).rgb * w1;
    rgb += textureSample(src_tex, src_samp, v.uv - dir).rgb * w1;
    rgb += textureSample(src_tex, src_samp, v.uv + dir * 2.0).rgb * w2;
    rgb += textureSample(src_tex, src_samp, v.uv - dir * 2.0).rgb * w2;
    return vec4<f32>(rgb, 1.0);
}
