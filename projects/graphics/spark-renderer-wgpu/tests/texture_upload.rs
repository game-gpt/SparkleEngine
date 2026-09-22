//! `TextureUpload` → wgpu 纹理创建烟测。

use spark_renderer_wgpu::{create_texture_from_upload, expected_mip_levels, map_texture_format};
use spark_texture::{MipmapPolicy, TextureFormat, TextureUpload};

fn test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_desc.backends = wgpu::Backends::PRIMARY;
    let instance = wgpu::Instance::new(instance_desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("texture-upload-smoke"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        memory_hints: Default::default(),
        experimental_features: Default::default(),
        trace: Default::default(),
    }))
    .ok()?;
    Some((device, queue))
}

#[test]
fn map_rgba8_formats() {
    assert!(matches!(map_texture_format(TextureFormat::Rgba8UnormSrgb).unwrap(), wgpu::TextureFormat::Rgba8UnormSrgb));
    assert!(matches!(map_texture_format(TextureFormat::Rgba8Unorm).unwrap(), wgpu::TextureFormat::Rgba8Unorm));
    assert!(matches!(map_texture_format(TextureFormat::Bc7RgbaUnorm).unwrap(), wgpu::TextureFormat::Bc7RgbaUnorm));
}

#[test]
fn upload_rgba8_srgb_with_cpu_mips() {
    let Some((device, queue)) = test_device()
    else {
        eprintln!("skip: no wgpu adapter");
        return;
    };
    let upload = TextureUpload::rgba8_srgb(4, 4, vec![255u8; 4 * 4 * 4])
        .unwrap()
        .with_debug_name("test-rgba8-mips");
    assert_eq!(expected_mip_levels(&upload), 3);
    let tex = create_texture_from_upload(&device, &queue, &upload).unwrap();
    assert_eq!(tex.mip_level_count(), 3);
    assert_eq!(tex.format(), wgpu::TextureFormat::Rgba8UnormSrgb);
    assert_eq!(tex.size().width, 4);
    assert_eq!(tex.size().height, 4);
}

#[test]
fn upload_rgba8_single_mip_no_generate() {
    let Some((device, queue)) = test_device()
    else {
        eprintln!("skip: no wgpu adapter");
        return;
    };
    let upload = TextureUpload::rgba8(2, 2, vec![0u8; 16], false)
        .unwrap()
        .with_mipmap(MipmapPolicy::None)
        .with_debug_name("test-rgba8-flat");
    assert_eq!(expected_mip_levels(&upload), 1);
    let tex = create_texture_from_upload(&device, &queue, &upload).unwrap();
    assert_eq!(tex.mip_level_count(), 1);
    assert_eq!(tex.format(), wgpu::TextureFormat::Rgba8Unorm);
}
