//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_gc::Value;
use spark_vm::*;

struct BufHost(String);
impl HostHooks for BufHost {
    fn print(&mut self, text: &str) {
        self.0.push_str(text);
        self.0.push('\n');
    }
}

#[test]
fn add_and_return() {
    let mut f = FuncProto::new("main", 0);
    let c1 = f.add_const_number(40.0);
    let c2 = f.add_const_number(2.0);
    f.emit(Op::LoadConst);
    f.emit_u16(c1);
    f.emit(Op::LoadConst);
    f.emit_u16(c2);
    f.emit(Op::Add);
    f.emit(Op::Return);
    let mut vm = Vm::new(Module { functions: vec![f], entry: 0, native_names: Vec::new() });
    let mut host = BufHost(String::new());
    let v = vm.run(&mut host).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn call_function_by_name() {
    let mut add = FuncProto::new("add", 2);
    add.locals = 2;
    add.emit(Op::LoadLocal);
    add.emit_u16(0);
    add.emit(Op::LoadLocal);
    add.emit_u16(1);
    add.emit(Op::Add);
    add.emit(Op::Return);
    let main = FuncProto::new("on_load", 0);
    let mut vm = Vm::new(Module { functions: vec![add, main], entry: 1, native_names: Vec::new() });
    let mut host = BufHost(String::new());
    let v = vm.call_function("add", &[Value::Number(40.0), Value::Number(2.0)], &mut host).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn call_host_slot_dispatches_by_prepared_name() {
    let mut f = FuncProto::new("on_load", 0);
    let c = f.add_const_number(21.0);
    f.emit(Op::LoadConst);
    f.emit_u16(c);
    f.emit(Op::CallHost);
    f.emit_u16(0);
    f.emit_u8(1);
    f.emit(Op::Return);
    let mut vm = Vm::new(Module { functions: vec![f], entry: 0, native_names: vec!["double".into()] });
    vm.prepare_host_slots(["double"]);
    vm.register_native("double", |_ctx, args| {
        let n = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
        Ok(Value::Number(n * 2.0))
    });
    let mut host = BufHost(String::new());
    let v = vm.run(&mut host).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
    assert_eq!(vm.call_hits.get("host:0:double"), Some(&1));
}

#[test]
fn host_call_limit_is_enforced() {
    let mut f = FuncProto::new("on_load", 0);
    f.emit(Op::CallHost);
    f.emit_u16(0);
    f.emit_u8(0);
    f.emit(Op::Return);
    let mut vm = Vm::new(Module { functions: vec![f], entry: 0, native_names: vec!["ping".into()] });
    vm.prepare_host_slots(["ping"]);
    vm.host_call_limit = 0;
    vm.register_native("ping", |_ctx, _args| Ok(Value::Null));
    let err = vm.run(&mut BufHost(String::new())).unwrap_err();
    assert!(matches!(err, VmError::HostCallLimitExceeded));
}

#[test]
fn step_limit_is_enforced() {
    let mut f = FuncProto::new("on_load", 0);
    // Jump 操作数读完后 ip=3，相对 -3 回到 Jump。
    f.emit(Op::Jump);
    f.emit_i16(-3);
    f.emit(Op::Return);
    let mut vm = Vm::new(Module { functions: vec![f], entry: 0, native_names: Vec::new() });
    vm.step_limit = 10;
    let err = vm.run(&mut BufHost(String::new())).unwrap_err();
    assert!(matches!(err, VmError::StepLimitExceeded), "got {err:?}");
}

#[test]
fn call_depth_limit_is_enforced() {
    let mut recur = FuncProto::new("recur", 0);
    let slot = recur.add_const_func(0);
    recur.emit(Op::LoadConst);
    recur.emit_u16(slot);
    recur.emit(Op::Call);
    recur.emit_u8(0);
    recur.emit(Op::Return);
    let mut main = FuncProto::new("on_load", 0);
    let slot = main.add_const_func(0);
    main.emit(Op::LoadConst);
    main.emit_u16(slot);
    main.emit(Op::Call);
    main.emit_u8(0);
    main.emit(Op::Return);
    let mut vm = Vm::new(Module { functions: vec![recur, main], entry: 1, native_names: Vec::new() });
    vm.call_depth_limit = 3;
    let err = vm.run(&mut BufHost(String::new())).unwrap_err();
    assert!(matches!(err, VmError::CallDepthExceeded));
}

#[test]
fn allocation_limit_is_enforced() {
    let mut f = FuncProto::new("on_load", 0);
    let s = f.add_string("x");
    // 循环：每次 LoadString 触发堆分配。
    f.emit(Op::LoadString);
    f.emit_u16(s);
    f.emit(Op::Pop);
    f.emit_u8(1);
    f.emit(Op::Jump);
    f.emit_i16(-8);
    f.emit(Op::Return);
    let mut vm = Vm::new(Module { functions: vec![f], entry: 0, native_names: Vec::new() });
    vm.allocation_limit = 5;
    vm.step_limit = 1_000_000;
    let err = vm.run(&mut BufHost(String::new())).unwrap_err();
    assert!(matches!(err, VmError::AllocationLimitExceeded), "got {err:?}");
}
