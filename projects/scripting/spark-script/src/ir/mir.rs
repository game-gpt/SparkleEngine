//! Spark MIR：语言无关的显式控制流 IR。

use std::sync::Arc;

use crate::host_schema::HostEffect;
use crate::ir::hir::Ty;
use crate::request::PackageId;

/// MIR 值引用（SSA 风格占位；槽分配可后续再定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirValue(pub u32);

/// 基本块终结。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirTerminator {
    Return {
        value: Option<MirValue>,
    },
    Jump {
        target: u32,
    },
    Branch {
        cond: MirValue,
        then_target: u32,
        else_target: u32,
    },
    Unreachable,
}

/// MIR 指令（非终结）。
#[derive(Debug, Clone, PartialEq)]
pub enum MirInst {
    Nop,
    ConstNull {
        dst: MirValue,
    },
    ConstBool {
        dst: MirValue,
        value: bool,
    },
    ConstNumber {
        dst: MirValue,
        value: f64,
    },
    ConstString {
        dst: MirValue,
        value: Arc<str>,
    },
    Move {
        dst: MirValue,
        src: MirValue,
    },
    Binary {
        dst: MirValue,
        op: crate::ir::hir::HirBinaryOp,
        lhs: MirValue,
        rhs: MirValue,
    },
    Unary {
        dst: MirValue,
        op: crate::ir::hir::HirUnaryOp,
        src: MirValue,
    },
    Call {
        dst: Option<MirValue>,
        func: MirValue,
        args: Vec<MirValue>,
    },
    HostCall {
        dst: Option<MirValue>,
        /// 链接前为短名；链接后应改为槽位。
        host_slot_or_name: HostRef,
        args: Vec<MirValue>,
    },
    DynamicSend {
        dst: Option<MirValue>,
        receiver: MirValue,
        method: Arc<str>,
        args: Vec<MirValue>,
    },
}

/// 宿主引用（链接前名字 / 链接后槽位）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostRef {
    Name(Arc<str>),
    Slot(u32),
}

/// 基本块。
#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: u32,
    pub insts: Vec<MirInst>,
    pub terminator: MirTerminator,
}

/// MIR 函数。
#[derive(Debug, Clone, PartialEq)]
pub struct MirFunction {
    pub name: Arc<str>,
    pub arity: u8,
    pub local_tys: Vec<Ty>,
    pub blocks: Vec<BasicBlock>,
    pub effects: Vec<HostEffect>,
}

/// MIR 模块。
#[derive(Debug, Clone, PartialEq)]
pub struct MirModule {
    pub package: PackageId,
    pub name: Arc<str>,
    pub functions: Vec<MirFunction>,
}
