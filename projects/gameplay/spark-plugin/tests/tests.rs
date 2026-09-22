//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_plugin::*;

use spark_gc::Value;
use spark_vm::{FuncProto, Module, NativeCtx, Op, StdHost, Vm};

struct EchoPlugin;

impl Plugin for EchoPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo { id: "echo", version: "0.0.0", description: "测试回声" }
    }

    fn native_names(&self) -> &'static [&'static str] {
        &["echo_ping"]
    }

    fn install(&mut self, vm: &mut Vm) {
        vm.register_native("echo_ping", |_ctx: &mut NativeCtx<'_>, args: Vec<Value>| Ok(args.into_iter().next().unwrap_or(Value::Null)));
    }
}

#[test]
fn register_and_call() {
    let mut reg = PluginRegistry::new();
    reg.register(Box::new(EchoPlugin)).unwrap();
    assert!(reg.contains("echo"));
    assert_eq!(reg.native_names(), ["echo_ping"]);

    let mut module = Module { functions: vec![], entry: 0, native_names: vec!["plugin.echo_ping".into()] };
    let mut f = FuncProto::new("on_load", 0);
    let c = f.add_const_number(7.0);
    f.emit(Op::LoadConst);
    f.emit_u16(c);
    f.emit(Op::CallHost);
    f.emit_u16(0);
    f.emit_u8(1);
    f.emit(Op::Return);
    module.functions.push(f);

    let mut vm = Vm::new(module);
    vm.prepare_host_slots(["plugin.echo_ping"]);
    reg.install_all(&mut vm);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(7.0));
}
