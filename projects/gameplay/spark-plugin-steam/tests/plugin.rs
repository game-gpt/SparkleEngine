//! 自 `src/plugin.rs` 迁出的原 `#[cfg(test)] mod tests`。
use std::rc::Rc;

use spark_plugin::PluginRegistry;
use spark_plugin_steam::*;
use spark_vm::{FuncProto, Module, Op, StdHost};

#[test]
fn null_backend_achievements_and_cloud() {
    let plugin = SteamPlugin::with_null_backend_app(480, "tester");
    let rt = Rc::clone(plugin.runtime());
    assert!(!rt.borrow().backend.is_available());
    assert_eq!(rt.borrow().backend.app_id(), 480);
    rt.borrow_mut().backend.unlock_achievement("ACH_FIRST").unwrap();
    assert!(rt.borrow().backend.is_achievement_unlocked("ACH_FIRST").unwrap());
    rt.borrow_mut().backend.set_stat("kills", 3.0).unwrap();
    assert!((rt.borrow().backend.get_stat("kills").unwrap() - 3.0).abs() < 1e-5);
    rt.borrow_mut().backend.cloud_write("save.txt", "hello").unwrap();
    assert_eq!(rt.borrow().backend.cloud_read("save.txt").unwrap().as_deref(), Some("hello"));
}

#[test]
fn install_and_call_app_id() {
    let mut reg = PluginRegistry::new();
    reg.register(Box::new(SteamPlugin::with_null_backend_app(1234, "u"))).unwrap();
    assert!(reg.contains("steam"));

    let mut module = Module { functions: vec![], entry: 0, native_names: vec!["plugin.steam_app_id".into()] };
    let mut f = FuncProto::new("on_load", 0);
    f.emit(Op::CallHost);
    f.emit_u16(0);
    f.emit_u8(0);
    f.emit(Op::Return);
    module.functions.push(f);

    let mut vm = spark_vm::Vm::new(module);
    vm.prepare_host_slots(["plugin.steam_app_id"]);
    reg.install_all(&mut vm);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(1234.0));
}
