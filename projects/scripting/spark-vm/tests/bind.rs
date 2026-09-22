//! 自 `src/bind.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_vm::*;

#[test]
fn accepts_call_host_only() {
    let mut f = FuncProto::new("on_load", 0);
    f.emit(Op::CallHost);
    f.emit_u16(0);
    f.emit_u8(0);
    f.emit(Op::Return);
    let module = Module { functions: vec![f], entry: 0, native_names: vec!["inc".into()] };
    reject_residual_call_native(&module).unwrap();
}

#[test]
fn rejects_call_native() {
    let mut f = FuncProto::new("on_load", 0);
    let si = f.add_string("inc");
    f.emit(Op::CallNative);
    f.emit_u16(si);
    f.emit_u8(0);
    f.emit(Op::Return);
    let module = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
    let err = reject_residual_call_native(&module).unwrap_err();
    assert!(err.contains("residual_call_native"), "{err}");
}
