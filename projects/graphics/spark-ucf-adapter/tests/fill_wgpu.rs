//! UCF Fill → wgpu RGBA8 纹理 staging 上传与读回。

use spark_ucf_adapter::{SparkComputePass, SparkUcfExecutor, WgpuRgba8Target, fill_rgba8_cpu_fallback};

fn headless_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .expect("wgpu adapter");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("spark-ucf-adapter test"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::default(),
        memory_hints: Default::default(),
        trace: Default::default(),
    }))
    .expect("wgpu device")
}

#[test]
fn color_fill_uploads_to_wgpu_texture() {
    let pass = SparkComputePass::color_fill("wgpu-clear", 4, 2, [10, 20, 30, 255]);
    let expected = fill_rgba8_cpu_fallback(pass.width, pass.height, pass.rgba);

    let (device, queue) = headless_device();
    let target = WgpuRgba8Target::new(&device, pass.width, pass.height, Some("ucf-fill"));
    let mut exec = SparkUcfExecutor::open_cpu();
    exec.run_color_fill_into_wgpu(&pass, &queue, &target)
        .expect("ucf → wgpu upload");

    let readback = WgpuRgba8Target::read_rgba8_back(&device, &queue, &target)
        .expect("wgpu readback");
    assert_eq!(readback, expected);

    let diag = exec.diagnostics();
    assert!(diag.contains_kind("readback"));
}
