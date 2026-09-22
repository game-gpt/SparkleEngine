//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_jit::*;
use spark_vm::{FuncProto, HostHooks, Module as VmModule, Op, Vm};

struct Nop;
impl HostHooks for Nop {
    fn print(&mut self, _: &str) {}
}

#[test]
fn fold_add() {
    let mut f = FuncProto::new("main", 0);
    let a = f.add_const_number(20.0);
    let b = f.add_const_number(22.0);
    f.emit(Op::LoadConst);
    f.emit_u16(a);
    f.emit(Op::LoadConst);
    f.emit_u16(b);
    f.emit(Op::Add);
    f.emit(Op::Return);
    specialize_func(&mut f).unwrap();
    // 应变为单 LoadConst + Return
    assert_eq!(f.code[0], Op::LoadConst as u8);
    assert_eq!(f.code[3], Op::Return as u8);
    let mut vm = Vm::new(VmModule { functions: vec![f], entry: 0, native_names: Vec::new() });
    let v = vm.run(&mut Nop).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn fold_div() {
    let mut f = FuncProto::new("main", 0);
    let a = f.add_const_number(84.0);
    let b = f.add_const_number(2.0);
    f.emit(Op::LoadConst);
    f.emit_u16(a);
    f.emit(Op::LoadConst);
    f.emit_u16(b);
    f.emit(Op::Div);
    f.emit(Op::Return);
    specialize_func(&mut f).unwrap();
    assert_eq!(f.code[0], Op::LoadConst as u8);
    assert_eq!(f.code[3], Op::Return as u8);
    let mut vm = Vm::new(VmModule { functions: vec![f], entry: 0, native_names: Vec::new() });
    let v = vm.run(&mut Nop).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn fold_mod() {
    let mut f = FuncProto::new("main", 0);
    let a = f.add_const_number(47.0);
    let b = f.add_const_number(5.0);
    f.emit(Op::LoadConst);
    f.emit_u16(a);
    f.emit(Op::LoadConst);
    f.emit_u16(b);
    f.emit(Op::Mod);
    f.emit(Op::Return);
    specialize_func(&mut f).unwrap();
    assert_eq!(f.code[0], Op::LoadConst as u8);
    assert_eq!(f.code[3], Op::Return as u8);
    let mut vm = Vm::new(VmModule { functions: vec![f], entry: 0, native_names: Vec::new() });
    let v = vm.run(&mut Nop).unwrap();
    assert_eq!(v.as_number(), Some(2.0));
}

#[test]
fn fold_cmp() {
    let mut f = FuncProto::new("main", 0);
    let a = f.add_const_number(3.0);
    let b = f.add_const_number(5.0);
    f.emit(Op::LoadConst);
    f.emit_u16(a);
    f.emit(Op::LoadConst);
    f.emit_u16(b);
    f.emit(Op::Lt);
    f.emit(Op::Return);
    specialize_func(&mut f).unwrap();
    assert_eq!(f.code[0], Op::LoadTrue as u8);
    assert_eq!(f.code[1], Op::Return as u8);
    let mut vm = Vm::new(VmModule { functions: vec![f], entry: 0, native_names: Vec::new() });
    let v = vm.run(&mut Nop).unwrap();
    assert!(v.truthy());
}
