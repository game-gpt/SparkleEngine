//! HIR → MIR 降低。

use crate::hir::{HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp};
use crate::mir::{
    BasicBlock, HostRef, IrEffect, MirFunction, MirInst, MirModule, MirTerminator, MirValue,
};

/// 将 HIR 模块降低为显式控制流 MIR。
pub fn lower_module(module: &HirModule) -> Result<MirModule, String> {
    let mut functions = Vec::with_capacity(module.functions.len());
    for func in &module.functions {
        functions.push(lower_function(func)?);
    }
    Ok(MirModule {
        package: module.package.clone(),
        name: module.name.clone(),
        functions,
    })
}

fn lower_function(func: &HirFunction) -> Result<MirFunction, String> {
    let mut local_tys = Vec::new();
    for (_, ty) in &func.params {
        local_tys.push(ty.clone());
    }
    for (_, ty) in &func.locals {
        local_tys.push(ty.clone());
    }

    let mut cx = LowerCx {
        next_value: 0,
        blocks: Vec::new(),
        current: 0,
        effects: Vec::new(),
    };
    cx.blocks.push(BasicBlock {
        id: 0,
        insts: Vec::new(),
        terminator: MirTerminator::Unreachable,
    });

    let mut saw_return = false;
    for stmt in &func.body {
        if matches!(stmt, HirStmt::Return { .. }) {
            saw_return = true;
        }
        lower_stmt(&mut cx, stmt)?;
        if matches!(
            cx.blocks[cx.current].terminator,
            MirTerminator::Return { .. }
        ) {
            // 后续语句不可达；若还有语句则开新块承接（简化：直接停）。
            break;
        }
    }
    if matches!(
        cx.blocks[cx.current].terminator,
        MirTerminator::Unreachable
    ) {
        if !saw_return {
            let v = cx.alloc();
            cx.emit(MirInst::ConstNull { dst: v });
            cx.set_term(MirTerminator::Return { value: Some(v) });
        }
    }

    Ok(MirFunction {
        name: func.name.clone(),
        arity: func.params.len() as u8,
        local_tys,
        blocks: cx.blocks,
        effects: cx.effects,
    })
}

struct LowerCx {
    next_value: u32,
    blocks: Vec<BasicBlock>,
    current: usize,
    effects: Vec<IrEffect>,
}

impl LowerCx {
    fn alloc(&mut self) -> MirValue {
        let v = MirValue(self.next_value);
        self.next_value += 1;
        v
    }

    fn emit(&mut self, inst: MirInst) {
        self.blocks[self.current].insts.push(inst);
    }

    fn set_term(&mut self, term: MirTerminator) {
        self.blocks[self.current].terminator = term;
    }

    fn new_block(&mut self) -> u32 {
        let id = self.blocks.len() as u32;
        self.blocks.push(BasicBlock {
            id,
            insts: Vec::new(),
            terminator: MirTerminator::Unreachable,
        });
        id
    }

    fn switch(&mut self, id: u32) {
        self.current = id as usize;
    }

    fn note_effect(&mut self, effect: IrEffect) {
        if !self.effects.contains(&effect) {
            self.effects.push(effect);
        }
    }

    fn term_is_open(&self) -> bool {
        matches!(
            self.blocks[self.current].terminator,
            MirTerminator::Unreachable
        )
    }
}

fn lower_stmt(cx: &mut LowerCx, stmt: &HirStmt) -> Result<(), String> {
    if !cx.term_is_open() {
        return Ok(());
    }
    match stmt {
        HirStmt::Expr { expr, .. } => {
            let _ = lower_expr(cx, expr)?;
            Ok(())
        }
        HirStmt::AssignLocal { index, value, .. } => {
            let src = lower_expr(cx, value)?;
            cx.emit(MirInst::StoreLocal {
                index: *index,
                src,
            });
            Ok(())
        }
        HirStmt::Return { value, .. } => {
            let v = match value {
                Some(e) => Some(lower_expr(cx, e)?),
                None => {
                    let n = cx.alloc();
                    cx.emit(MirInst::ConstNull { dst: n });
                    Some(n)
                }
            };
            cx.set_term(MirTerminator::Return { value: v });
            Ok(())
        }
        HirStmt::If {
            cond,
            then_body,
            else_body,
            ..
        } => {
            let c = lower_expr(cx, cond)?;
            let then_id = cx.new_block();
            let else_id = cx.new_block();
            let join_id = cx.new_block();
            cx.set_term(MirTerminator::Branch {
                cond: c,
                then_target: then_id,
                else_target: else_id,
            });
            cx.switch(then_id);
            for s in then_body {
                lower_stmt(cx, s)?;
            }
            if cx.term_is_open() {
                cx.set_term(MirTerminator::Jump { target: join_id });
            }
            cx.switch(else_id);
            for s in else_body {
                lower_stmt(cx, s)?;
            }
            if cx.term_is_open() {
                cx.set_term(MirTerminator::Jump { target: join_id });
            }
            cx.switch(join_id);
            Ok(())
        }
        HirStmt::While { cond, body, .. } => {
            let header = cx.new_block();
            let body_id = cx.new_block();
            let exit = cx.new_block();
            cx.set_term(MirTerminator::Jump { target: header });
            cx.switch(header);
            let c = lower_expr(cx, cond)?;
            cx.set_term(MirTerminator::Branch {
                cond: c,
                then_target: body_id,
                else_target: exit,
            });
            cx.switch(body_id);
            for s in body {
                lower_stmt(cx, s)?;
            }
            if cx.term_is_open() {
                cx.set_term(MirTerminator::Jump { target: header });
            }
            cx.switch(exit);
            Ok(())
        }
    }
}

fn lower_expr(cx: &mut LowerCx, expr: &HirExpr) -> Result<MirValue, String> {
    match expr {
        HirExpr::LiteralNull { .. } => {
            let dst = cx.alloc();
            cx.emit(MirInst::ConstNull { dst });
            Ok(dst)
        }
        HirExpr::LiteralBool { value, .. } => {
            let dst = cx.alloc();
            cx.emit(MirInst::ConstBool {
                dst,
                value: *value,
            });
            Ok(dst)
        }
        HirExpr::LiteralNumber { value, .. } => {
            let dst = cx.alloc();
            cx.emit(MirInst::ConstNumber {
                dst,
                value: *value,
            });
            Ok(dst)
        }
        HirExpr::LiteralString { value, .. } => {
            let dst = cx.alloc();
            cx.emit(MirInst::ConstString {
                dst,
                value: value.clone(),
            });
            Ok(dst)
        }
        HirExpr::Local { index, .. } => {
            let dst = cx.alloc();
            cx.emit(MirInst::LoadLocal {
                dst,
                index: *index,
            });
            Ok(dst)
        }
        HirExpr::FuncRef { func_index, .. } => {
            let dst = cx.alloc();
            cx.emit(MirInst::ConstFunc {
                dst,
                func_index: *func_index,
            });
            Ok(dst)
        }
        HirExpr::Binary { op, lhs, rhs, .. } => {
            let l = lower_expr(cx, lhs)?;
            let r = lower_expr(cx, rhs)?;
            let dst = cx.alloc();
            cx.emit(MirInst::Binary {
                dst,
                op: *op,
                lhs: l,
                rhs: r,
            });
            Ok(dst)
        }
        HirExpr::Unary { op, expr, .. } => {
            let src = lower_expr(cx, expr)?;
            let dst = cx.alloc();
            cx.emit(MirInst::Unary {
                dst,
                op: *op,
                src,
            });
            Ok(dst)
        }
        HirExpr::ToBool { expr, .. } => {
            let src = lower_expr(cx, expr)?;
            let dst = cx.alloc();
            cx.emit(MirInst::Unary {
                dst,
                op: HirUnaryOp::Not,
                src,
            });
            let dst2 = cx.alloc();
            cx.emit(MirInst::Unary {
                dst: dst2,
                op: HirUnaryOp::Not,
                src: dst,
            });
            Ok(dst2)
        }
        HirExpr::Print { value, .. } => {
            let src = lower_expr(cx, value)?;
            cx.emit(MirInst::Print { src });
            Ok(src)
        }
        HirExpr::HostCall {
            host_name, args, ..
        } => {
            cx.note_effect(IrEffect::HostCall);
            let mut argv = Vec::with_capacity(args.len());
            for a in args {
                argv.push(lower_expr(cx, a)?);
            }
            let dst = cx.alloc();
            cx.emit(MirInst::HostCall {
                dst: Some(dst),
                host_slot_or_name: HostRef::Name(host_name.clone()),
                args: argv,
            });
            Ok(dst)
        }
        HirExpr::Call { callee, args, .. } => {
            let func = lower_expr(cx, callee)?;
            let mut argv = Vec::with_capacity(args.len());
            for a in args {
                argv.push(lower_expr(cx, a)?);
            }
            let dst = cx.alloc();
            cx.emit(MirInst::Call {
                dst: Some(dst),
                func,
                args: argv,
            });
            Ok(dst)
        }
        HirExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let c = lower_expr(cx, cond)?;
            let then_id = cx.new_block();
            let else_id = cx.new_block();
            let join_id = cx.new_block();
            let result = cx.alloc();
            cx.set_term(MirTerminator::Branch {
                cond: c,
                then_target: then_id,
                else_target: else_id,
            });
            cx.switch(then_id);
            let tv = lower_expr(cx, then_branch)?;
            cx.emit(MirInst::Move {
                dst: result,
                src: tv,
            });
            cx.set_term(MirTerminator::Jump { target: join_id });
            cx.switch(else_id);
            let ev = lower_expr(cx, else_branch)?;
            cx.emit(MirInst::Move {
                dst: result,
                src: ev,
            });
            cx.set_term(MirTerminator::Jump { target: join_id });
            cx.switch(join_id);
            Ok(result)
        }
        HirExpr::Block {
            stmts, result, ..
        } => {
            for s in stmts {
                lower_stmt(cx, s)?;
                if !cx.term_is_open() {
                    let dummy = cx.alloc();
                    cx.emit(MirInst::ConstNull { dst: dummy });
                    return Ok(dummy);
                }
            }
            match result {
                Some(e) => lower_expr(cx, e),
                None => {
                    let dst = cx.alloc();
                    cx.emit(MirInst::ConstNull { dst });
                    Ok(dst)
                }
            }
        }
        HirExpr::DynamicSend { .. } => Err(format!("unsupported_hir_expr_in_lower:{expr:?}")),
    }
}
