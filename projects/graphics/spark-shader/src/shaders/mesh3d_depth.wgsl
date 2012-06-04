// 深度专用：仅写阴影图深度，无颜色输出。

struct ObjectUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> object: ObjectUniforms;

@vertex
fn vs_main(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    let world = object.model * vec4<f32>(pos, 1.0);
    return object.view_proj * world;
}
