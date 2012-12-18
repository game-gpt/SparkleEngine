//! Spark MIR：语言无关的显式控制流 IR。

use std::sync::Arc;

use crate::{
    hir::{HirBinaryOp, HirUnaryOp, PackageId, Ty},
    host::HostId,
};

/// MIR 层效果标记（与宿主 schema 效果对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrEffect {
    Pure,
    HostCall,
    ReadWorld,
    WriteComponent,
    SpawnEntity,
    DespawnEntity,
    AssetRead,
    NetworkSend,
    Nondeterministic,
    Suspend,
    EditorOnly,
    Dynamic,
}

impl IrEffect {
    pub fn from_host(kind: crate::host::HostEffectKind) -> Self {
        match kind {
            crate::host::HostEffectKind::Pure => Self::Pure,
            crate::host::HostEffectKind::ReadWorld => Self::ReadWorld,
            crate::host::HostEffectKind::WriteComponent => Self::WriteComponent,
            crate::host::HostEffectKind::SpawnEntity => Self::SpawnEntity,
            crate::host::HostEffectKind::DespawnEntity => Self::DespawnEntity,
            crate::host::HostEffectKind::AssetRead => Self::AssetRead,
            crate::host::HostEffectKind::AudioEmit => Self::Nondeterministic,
            crate::host::HostEffectKind::NetworkSend => Self::NetworkSend,
            crate::host::HostEffectKind::Nondeterministic => Self::Nondeterministic,
            crate::host::HostEffectKind::Suspend => Self::Suspend,
            crate::host::HostEffectKind::EditorOnly => Self::EditorOnly,
        }
    }
}

/// MIR 值引用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirValue(pub u32);

/// 基本块终结。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirTerminator {
    Return { value: Option<MirValue> },
    Jump { target: u32 },
    Branch { cond: MirValue, then_target: u32, else_target: u32 },
    Unreachable,
}

/// MIR 指令（非终结）。
#[derive(Debug, Clone, PartialEq)]
pub enum MirInst {
    Nop,
    ConstNull { dst: MirValue },
    ConstBool { dst: MirValue, value: bool },
    ConstNumber { dst: MirValue, value: f64 },
    ConstString { dst: MirValue, value: Arc<str> },
    ConstFunc { dst: MirValue, func_index: u32 },
    LoadLocal { dst: MirValue, index: u32 },
    StoreLocal { index: u32, src: MirValue },
    Move { dst: MirValue, src: MirValue },
    Binary { dst: MirValue, op: HirBinaryOp, lhs: MirValue, rhs: MirValue },
    Unary { dst: MirValue, op: HirUnaryOp, src: MirValue },
    Call { dst: Option<MirValue>, func: MirValue, args: Vec<MirValue> },
    Print { src: MirValue },
    HostCall { dst: Option<MirValue>, host_slot_or_name: HostRef, args: Vec<MirValue> },
    DynamicSend { dst: Option<MirValue>, receiver: MirValue, method: Arc<str>, args: Vec<MirValue> },
}

/// 宿主引用（降低后身份 / 已解析槽位）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostRef {
    /// 稳定身份；codegen 按 [`crate::HostBindTable`] 解析槽位。
    Id(HostId),
    /// 已解析槽位（链接或显式绑定后）。
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
    pub effects: Vec<IrEffect>,
}

/// MIR 模块。
#[derive(Debug, Clone, PartialEq)]
pub struct MirModule {
    pub package: PackageId,
    pub name: Arc<str>,
    pub functions: Vec<MirFunction>,
}
