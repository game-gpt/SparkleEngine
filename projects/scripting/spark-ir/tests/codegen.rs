//! 自 `src/codegen.rs` 迁出的原 `#[cfg(test)] mod tests`。
use std::sync::Arc;

use spark_ir::{
    HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, HostBindTable, HostEmitMode, HostId, PackageId, Ty, emit_module,
    emit_module_with_host, lower_module,
};
use spark_vm::{Op, StdHost, Vm};

#[test]
fn arithmetic_via_ir_pipeline() {
    let hir = HirModule {
        package: PackageId::anonymous(),
        name: Arc::from("main"),
        functions: vec![HirFunction {
            name: Arc::from("on_load"),
            symbol: None,
            params: Vec::new(),
            return_ty: Ty::Float,
            locals: Vec::new(),
            body: vec![HirStmt::Return {
                value: Some(HirExpr::Binary {
                    op: HirBinaryOp::Add,
                    lhs: Box::new(HirExpr::LiteralNumber { value: 40.0, span: None }),
                    rhs: Box::new(HirExpr::LiteralNumber { value: 2.0, span: None }),
                    span: None,
                }),
                span: None,
            }],
            span: None,
        }],
    };
    let mir = lower_module(&hir).unwrap();
    let module = emit_module(&mir).unwrap();
    let mut vm = Vm::new(module);
    let mut host = StdHost;
    let v = vm.run(&mut host).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn call_via_ir_pipeline() {
    let hir = HirModule {
        package: PackageId::anonymous(),
        name: Arc::from("main"),
        functions: vec![
            HirFunction {
                name: Arc::from("add"),
                symbol: None,
                params: vec![(Arc::from("a"), Ty::Float), (Arc::from("b"), Ty::Float)],
                return_ty: Ty::Float,
                locals: Vec::new(),
                body: vec![HirStmt::Return {
                    value: Some(HirExpr::Binary {
                        op: HirBinaryOp::Add,
                        lhs: Box::new(HirExpr::Local { index: 0, span: None }),
                        rhs: Box::new(HirExpr::Local { index: 1, span: None }),
                        span: None,
                    }),
                    span: None,
                }],
                span: None,
            },
            HirFunction {
                name: Arc::from("on_load"),
                symbol: None,
                params: Vec::new(),
                return_ty: Ty::Float,
                locals: Vec::new(),
                body: vec![HirStmt::Return {
                    value: Some(HirExpr::Call {
                        callee: Box::new(HirExpr::FuncRef { func_index: 0, span: None }),
                        args: vec![HirExpr::LiteralNumber { value: 40.0, span: None }, HirExpr::LiteralNumber { value: 2.0, span: None }],
                        span: None,
                    }),
                    span: None,
                }],
                span: None,
            },
        ],
    };
    let mir = lower_module(&hir).unwrap();
    let module = emit_module(&mir).unwrap();
    let mut vm = Vm::new(module);
    let mut host = StdHost;
    let v = vm.run(&mut host).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn host_call_emits_call_host_slots() {
    let hir = HirModule {
        package: PackageId::anonymous(),
        name: Arc::from("main"),
        functions: vec![HirFunction {
            name: Arc::from("on_load"),
            symbol: None,
            params: Vec::new(),
            return_ty: Ty::Float,
            locals: Vec::new(),
            body: vec![HirStmt::Return {
                value: Some(HirExpr::HostCall {
                    host: HostId::new("host", "double", 1),
                    args: vec![HirExpr::LiteralNumber { value: 21.0, span: None }],
                    effects: Vec::new(),
                    span: None,
                }),
                span: None,
            }],
            span: None,
        }],
    };
    let mir = lower_module(&hir).unwrap();
    let binds = HostBindTable::from_ids([HostId::new("host", "double", 1)]).unwrap();
    let module = emit_module_with_host(&mir, HostEmitMode::Bound(&binds)).unwrap();
    assert!(module.functions[0].code.iter().any(|&b| b == Op::CallHost as u8));
    let mut vm = Vm::new(module);
    vm.prepare_host_slots(["host.double"]);
    vm.register_native("host.double", |_ctx, args| {
        let n = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
        Ok(spark_gc::Value::Number(n * 2.0))
    });
    let mut host = StdHost;
    let v = vm.run(&mut host).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}
