//! UCF Fill → DrawList 纹理上传队列。

use spark_renderer::DrawList;
use spark_types::Color;
use spark_ucf_adapter::{enqueue_color_fill_texture, fill_rgba8_cpu_fallback, SparkComputePass, SparkUcfExecutor};

#[test]
fn color_fill_enqueues_drawlist_texture() {
    let pass = SparkComputePass::color_fill("drawlist-clear", 3, 2, [5, 6, 7, 255]);
    let expected = fill_rgba8_cpu_fallback(pass.width, pass.height, pass.rgba);

    let mut exec = SparkUcfExecutor::open_cpu();
    let mut draw = DrawList::new(Color::rgb(0.0, 0.0, 0.0));
    let id = enqueue_color_fill_texture(&mut draw, &mut exec, &pass).expect("enqueue");
    assert_eq!(draw.texture_uploads.len(), 1);
    assert_eq!(draw.texture_uploads[0].0, id);
    let upload = &draw.texture_uploads[0].1;
    assert_eq!(upload.desc.width, pass.width);
    assert_eq!(upload.desc.height, pass.height);
    assert_eq!(upload.data.bytes.as_ref(), expected.as_slice());
}
