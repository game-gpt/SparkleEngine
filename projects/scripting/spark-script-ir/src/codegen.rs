//! MIR → `spark-vm::Module` 字节码生成。

use spark_vm::{FuncProto, Module, Op};

use crate::hir::{HirBinaryOp, HirUnaryOp};
use crate::mir::{HostRef, MirFunction, MirInst, MirModule, MirTerminator, MirValue};

/// 将已验证语义的 MIR 发射为 VM 模块。
pub fn emit_module(module: &MirModule) -> Result<Module, String> {
    let native_names = collect_native_names(module);
    let mut functions = Vec::with_capacity(module.functions.len());
    let mut entry = 0usize;
    for (i, f) in module.functions.iter().enumerate() {
        if f.name.as_ref() == "__main" {
            entry = i;
        }
        functions.push(emit_function(f, &native_names)?);
    }
    if functions.is_empty() {
        return Err("empty_mir_module".into());
    }
    Ok(Module {
        functions,
        entry,
        native_names,
    })
}

fn collect_native_names(module: &MirModule) -> Vec<String> {
    let mut names = Vec::new();
    for f in &module.functions {
        for b in &f.blocks {
            for inst in &b.insts {
                if let MirInst::HostCall {
                    host_slot_or_name: HostRef::Name(n),
                    ..
                } = inst
                {
                    if !names.iter().any(|x| x == n.as_ref()) {
                        names.push(n.as_ref().to_string());
                    }
                }
            }
        }
    }
    names
}

fn emit_function(func: &MirFunction, native_names: &[String]) -> Result<FuncProto, String> {
    let mut f = FuncProto::new(func.name.as_ref(), func.arity);
    f.locals = func.local_tys.len().max(func.arity as usize) as u16;

    let mut block_offsets: Vec<usize> = Vec::with_capacity(func.blocks.len());
    let mut pending_jumps: Vec<(usize, u32)> = Vec::new();

    let value_base = f.locals;
    let mut max_val: u32 = 0;
    for b in &func.blocks {
        for inst in &b.insts {
            max_val = max_val.max(inst_max_value(inst));
        }
        match &b.terminator {
            MirTerminator::Return { value: Some(v) } => max_val = max_val.max(v.0),
            MirTerminator::Branch { cond, .. } => max_val = max_val.max(cond.0),
            _ => {}
        }
    }
    let slot_of = |v: MirValue| -> u16 { value_base + v.0 as u16 };
    f.locals = value_base + max_val as u16 + 1;

    for block in &func.blocks {
        block_offsets.push(f.len());
        for inst in &block.insts {
            emit_inst(&mut f, inst, &slot_of, native_names)?;
        }
        match &block.terminator {
            MirTerminator::Return { value } => {
                match value {
                    Some(v) => {
                        f.emit(Op::LoadLocal);
                        f.emit_u16(slot_of(*v));
                    }
                    None => f.emit(Op::LoadNull),
                }
                f.emit(Op::Return);
            }
            MirTerminator::Jump { target } => {
                f.emit(Op::Jump);
                pending_jumps.push((f.len(), *target));
                f.emit_i16(0);
            }
            MirTerminator::Branch {
                cond,
                then_target,
                else_target,
            } => {
                f.emit(Op::LoadLocal);
                f.emit_u16(slot_of(*cond));
                f.emit(Op::JumpIfFalse);
                pending_jumps.push((f.len(), *else_target));
                f.emit_i16(0);
                f.emit(Op::Jump);
                pending_jumps.push((f.len(), *then_target));
                f.emit_i16(0);
            }
            MirTerminator::Unreachable => {
                f.emit(Op::LoadNull);
                f.emit(Op::Return);
            }
        }
    }

    for (at, target) in pending_jumps {
        let dest = block_offsets[target as usize];
        let rel = (dest as isize) - ((at + 2) as isize);
        f.patch_i16(at, rel as i16);
    }

    Ok(f)
}

fn inst_max_value(inst: &MirInst) -> u32 {
    match inst {
        MirInst::Nop => 0,
        MirInst::ConstNull { dst }
        | MirInst::ConstBool { dst, .. }
        | MirInst::ConstNumber { dst, .. }
        | MirInst::ConstString { dst, .. }
        | MirInst::ConstFunc { dst, .. }
        | MirInst::LoadLocal { dst, .. } => dst.0,
        MirInst::StoreLocal { src, .. } | MirInst::Print { src } => src.0,
        MirInst::Move { dst, src } => dst.0.max(src.0),
        MirInst::Binary { dst, lhs, rhs, .. } => dst.0.max(lhs.0).max(rhs.0),
        MirInst::Unary { dst, src, .. } => dst.0.max(src.0),
        MirInst::Call { dst, func, args } => {
            let mut m = func.0;
            if let Some(d) = dst {
                m = m.max(d.0);
            }
            for a in args {
                m = m.max(a.0);
            }
            m
        }
        MirInst::HostCall { dst, args, .. } => {
            let mut m = 0;
            if let Some(d) = dst {
                m = d.0;
            }
            for a in args {
                m = m.max(a.0);
            }
            m
        }
        MirInst::DynamicSend {
            dst,
            receiver,
            args,
            ..
        } => {
            let mut m = receiver.0;
            if let Some(d) = dst {
                m = m.max(d.0);
            }
            for a in args {
                m = m.max(a.0);
            }
            m
        }
    }
}

fn emit_inst(
    f: &mut FuncProto,
    inst: &MirInst,
    slot_of: &dyn Fn(MirValue) -> u16,
    native_names: &[String],
) -> Result<(), String> {
    match inst {
        MirInst::Nop => {}
        MirInst::ConstNull { dst } => {
            f.emit(Op::LoadNull);
            store(f, slot_of(*dst));
        }
        MirInst::ConstBool { dst, value } => {
            if *value {
                f.emit(Op::LoadTrue);
            } else {
                f.emit(Op::LoadFalse);
            }
            store(f, slot_of(*dst));
        }
        MirInst::ConstNumber { dst, value } => {
            let i = f.add_const_number(*value);
            f.emit(Op::LoadConst);
            f.emit_u16(i);
            store(f, slot_of(*dst));
        }
        MirInst::ConstString { dst, value } => {
            let i = f.add_string(value.as_ref());
            f.emit(Op::LoadString);
            f.emit_u16(i);
            store(f, slot_of(*dst));
        }
        MirInst::ConstFunc { dst, func_index } => {
            let i = f.add_const_func(*func_index);
            f.emit(Op::LoadConst);
            f.emit_u16(i);
            store(f, slot_of(*dst));
        }
        MirInst::LoadLocal { dst, index } => {
            f.emit(Op::LoadLocal);
            f.emit_u16(*index as u16);
            store(f, slot_of(*dst));
        }
        MirInst::StoreLocal { index, src } => {
            f.emit(Op::LoadLocal);
            f.emit_u16(slot_of(*src));
            f.emit(Op::StoreLocal);
            f.emit_u16(*index as u16);
        }
        MirInst::Move { dst, src } => {
            f.emit(Op::LoadLocal);
            f.emit_u16(slot_of(*src));
            store(f, slot_of(*dst));
        }
        MirInst::Binary { dst, op, lhs, rhs } => {
            f.emit(Op::LoadLocal);
            f.emit_u16(slot_of(*lhs));
            f.emit(Op::LoadLocal);
            f.emit_u16(slot_of(*rhs));
            f.emit(bin_op(*op));
            store(f, slot_of(*dst));
        }
        MirInst::Unary { dst, op, src } => {
            f.emit(Op::LoadLocal);
            f.emit_u16(slot_of(*src));
            f.emit(unary_op(*op));
            store(f, slot_of(*dst));
        }
        MirInst::Print { src } => {
            f.emit(Op::LoadLocal);
            f.emit_u16(slot_of(*src));
            f.emit(Op::Print);
        }
        MirInst::Call { dst, func, args } => {
            f.emit(Op::LoadLocal);
            f.emit_u16(slot_of(*func));
            for a in args {
                f.emit(Op::LoadLocal);
                f.emit_u16(slot_of(*a));
            }
            f.emit(Op::Call);
            f.emit_u8(args.len() as u8);
            if let Some(d) = dst {
                store(f, slot_of(*d));
            } else {
                f.emit(Op::Pop);
                f.emit_u8(1);
            }
        }
        MirInst::HostCall {
            dst,
            host_slot_or_name,
            args,
        } => {
            for a in args {
                f.emit(Op::LoadLocal);
                f.emit_u16(slot_of(*a));
            }
            let name = match host_slot_or_name {
                HostRef::Name(n) => n.as_ref(),
                HostRef::Slot(s) => return Err(format!("host_slot_not_named:{s}")),
            };
            if !native_names.iter().any(|n| n == name) {
                return Err(format!("native_missing:{name}"));
            }
            let si = f.add_string(name);
            f.emit(Op::CallNative);
            f.emit_u16(si);
            f.emit_u8(args.len() as u8);
            if let Some(d) = dst {
                store(f, slot_of(*d));
            } else {
                f.emit(Op::Pop);
                f.emit_u8(1);
            }
        }
        MirInst::DynamicSend { .. } => {
            Err(format!("unsupported_mir_inst:{inst:?}"))?
        }
    }
    Ok(())
}

fn store(f: &mut FuncProto, slot: u16) {
    f.emit(Op::StoreLocal);
    f.emit_u16(slot);
}

fn bin_op(op: HirBinaryOp) -> Op {
    match op {
        HirBinaryOp::Add => Op::Add,
        HirBinaryOp::Sub => Op::Sub,
        HirBinaryOp::Mul => Op::Mul,
        HirBinaryOp::Div => Op::Div,
        HirBinaryOp::Mod => Op::Mod,
        HirBinaryOp::Eq => Op::Eq,
        HirBinaryOp::Ne => Op::Ne,
        HirBinaryOp::Lt => Op::Lt,
        HirBinaryOp::Le => Op::Le,
        HirBinaryOp::Gt => Op::Gt,
        HirBinaryOp::Ge => Op::Ge,
    }
}

fn unary_op(op: HirUnaryOp) -> Op {
    match op {
        HirUnaryOp::Neg => Op::Neg,
        HirUnaryOp::Not => Op::Not,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::hir::{HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, PackageId, Ty};
    use crate::lower::lower_module;
    use spark_vm::{StdHost, Vm};

    #[test]
    fn arithmetic_via_ir_pipeline() {
        let hir = HirModule {
            package: PackageId::anonymous(),
            name: Arc::from("main"),
            functions: vec![HirFunction {
                name: Arc::from("__main"),
                symbol: None,
                params: Vec::new(),
                return_ty: Ty::Float,
                locals: Vec::new(),
                body: vec![HirStmt::Return {
                    value: Some(HirExpr::Binary {
                        op: HirBinaryOp::Add,
                        lhs: Box::new(HirExpr::LiteralNumber {
                            value: 40.0,
                            span: None,
                        }),
                        rhs: Box::new(HirExpr::LiteralNumber {
                            value: 2.0,
                            span: None,
                        }),
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
                    params: vec![
                        (Arc::from("a"), Ty::Float),
                        (Arc::from("b"), Ty::Float),
                    ],
                    return_ty: Ty::Float,
                    locals: Vec::new(),
                    body: vec![HirStmt::Return {
                        value: Some(HirExpr::Binary {
                            op: HirBinaryOp::Add,
                            lhs: Box::new(HirExpr::Local {
                                index: 0,
                                span: None,
                            }),
                            rhs: Box::new(HirExpr::Local {
                                index: 1,
                                span: None,
                            }),
                            span: None,
                        }),
                        span: None,
                    }],
                    span: None,
                },
                HirFunction {
                    name: Arc::from("__main"),
                    symbol: None,
                    params: Vec::new(),
                    return_ty: Ty::Float,
                    locals: Vec::new(),
                    body: vec![HirStmt::Return {
                        value: Some(HirExpr::Call {
                            callee: Box::new(HirExpr::FuncRef {
                                func_index: 0,
                                span: None,
                            }),
                            args: vec![
                                HirExpr::LiteralNumber {
                                    value: 40.0,
                                    span: None,
                                },
                                HirExpr::LiteralNumber {
                                    value: 2.0,
                                    span: None,
                                },
                            ],
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
}
