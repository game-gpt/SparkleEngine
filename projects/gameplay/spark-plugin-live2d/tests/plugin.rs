//! 自 `src/plugin.rs` 迁出的原 `#[cfg(test)] mod tests`。
use std::rc::Rc;

use spark_plugin::PluginRegistry;
use spark_plugin_live2d::*;
use spark_vm::{FuncProto, Module, Op, StdHost};

#[test]
fn null_backend_params() {
    let plugin = Live2dPlugin::with_null_backend();
    let rt = Rc::clone(plugin.runtime());
    let id = rt.borrow_mut().backend.load("model.model3.json").unwrap();
    rt.borrow_mut().backend.set_param(id, "ParamAngleX", 0.5).unwrap();
    let v = rt.borrow().backend.get_param(id, "ParamAngleX").unwrap();
    assert!((v - 0.5).abs() < 1e-5);
}

#[test]
fn install_and_load_via_vm() {
    let mut reg = PluginRegistry::new();
    reg.register(Box::new(Live2dPlugin::with_null_backend())).unwrap();
    assert!(reg.contains("live2d"));

    let mut module = Module { functions: vec![], entry: 0, native_names: vec!["plugin.live2d_load".into()] };
    let mut f = FuncProto::new("on_load", 0);
    let s = f.add_string("demo");
    f.emit(Op::LoadString);
    f.emit_u16(s);
    f.emit(Op::CallHost);
    f.emit_u16(0);
    f.emit_u8(1);
    f.emit(Op::Return);
    module.functions.push(f);

    let mut vm = spark_vm::Vm::new(module);
    vm.prepare_host_slots(["plugin.live2d_load"]);
    reg.install_all(&mut vm);
    let idv = vm.run(&mut StdHost).unwrap();
    assert_eq!(idv.as_number(), Some(0.0));
}
