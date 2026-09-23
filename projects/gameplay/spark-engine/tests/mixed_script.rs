//! 统一调度图内真实脚本字节码与原生 System 混排。

use spark_engine::SparkRuntime;
use spark_input::Input;
use spark_renderer::FrameCtx;
use spark_vm::StdHost;

fn mixed_spawn_mods_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join("spark_mixed_script_mods");
    let mod_dir = root.join("mixed_spawn");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&mod_dir).unwrap();
    std::fs::write(
        mod_dir.join("mod.von"),
        r#"id = "mixed_spawn"
version = "0.0.0"
entry = "main.vk"
"#,
    )
    .unwrap();
    std::fs::write(
        mod_dir.join("main.vk"),
        r#"
        micro on_load() {
            return 0
        }
        micro update() {
            queue_spawn("rock")
            return 0
        }
        return 0
        "#,
    )
    .unwrap();
    root
}

fn frame_ctx(input: &Input, dt: f32) -> FrameCtx<'_> {
    FrameCtx { input, dt, screen_w: 640.0, screen_h: 480.0, dpi_scale: 1.0, timing: Default::default() }
}

#[test]
fn mixed_phase_script_bytecode_spawns_via_tick_sim() {
    let mods_root = mixed_spawn_mods_root();
    let mut runtime = SparkRuntime::new();
    runtime.load_script_package(&mods_root).unwrap();
    let input = Input::default();
    let frame = frame_ctx(&input, 0.016);
    let mut host = StdHost;
    runtime.tick_sim(&frame, &mut host).unwrap();
    assert_eq!(runtime.world().entity_count(), 1);
}
