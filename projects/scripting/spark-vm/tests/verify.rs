//! 自 `src/verify.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_vm::*;

use spark_vm::{FuncProto, Op};

#[test]
fn rejects_unknown_opcode() {
    let mut f = FuncProto::new("main", 0);
    f.code.push(255);
    let m = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
    let err = verify_bytecode(&m).unwrap_err();
    assert!(matches!(err, BytecodeVerifyError::UnknownOpcode { .. }));
}

#[test]
fn rejects_jump_into_operand() {
    let mut f = FuncProto::new("main", 0);
    f.emit(Op::Jump);
    // 相对跳转到操作数中间：ip after i16 = 3, target = 3 + (-2) = 1（落在 i16 上）
    f.emit_i16(-2);
    f.emit(Op::LoadNull);
    f.emit(Op::Return);
    let m = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
    let err = verify_bytecode(&m).unwrap_err();
    assert!(matches!(err, BytecodeVerifyError::JumpOffBoundary { .. }));
}

#[test]
fn accepts_simple_main() {
    let mut f = FuncProto::new("main", 0);
    f.emit(Op::LoadNull);
    f.emit(Op::Return);
    let m = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
    verify_bytecode(&m).unwrap();
}

#[test]
fn rejects_host_slot_oob() {
    let mut f = FuncProto::new("main", 0);
    f.emit(Op::LoadNull);
    f.emit(Op::CallHost);
    f.emit_u16(99);
    f.emit_u8(1);
    f.emit(Op::Return);
    let m = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
    let err = verify_bytecode_with_host(&m, 2).unwrap_err();
    assert!(matches!(err, BytecodeVerifyError::HostSlotOob { slot: 99, len: 2, .. }));
}

#[test]
fn sealed_rejects_residual_call_native() {
    let mut f = FuncProto::new("main", 0);
    let si = f.add_string("print");
    f.emit(Op::LoadNull);
    f.emit(Op::CallNative);
    f.emit_u16(si);
    f.emit_u8(1);
    f.emit(Op::Return);
    let m = Module { functions: vec![f], entry: 0, native_names: vec!["print".into()] };
    let err = verify_bytecode_with_host(&m, 1).unwrap_err();
    assert!(matches!(err, BytecodeVerifyError::ResidualCallNative { .. }));
}
