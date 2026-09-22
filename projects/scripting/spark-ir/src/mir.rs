//! Spark MIR：语言无关的显式控制流 IR。

use std::sync::Arc;

use crate::{
    hir::{HirBinaryOp, HirUnaryOp, PackageId, Ty},
    host::HostId,
};

/// MIR 层效果标记（与宿主 schema 效果对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrEffect {
    /// 纯计算，无世界副作用。
    Pure,
    /// 调用宿主（具体效果见绑定表）。
    HostCall,
    /// 只读世界。
    ReadWorld,
    /// 写组件。
    WriteComponent,
    /// 生成实体。
    SpawnEntity,
    /// 销毁实体。
    DespawnEntity,
    /// 读资产。
    AssetRead,
    /// 网络发送。
    NetworkSend,
    /// 非确定性。
    Nondeterministic,
    /// 挂起 / await。
    Suspend,
    /// 仅编辑器。
    EditorOnly,
    /// 动态派发（效果在运行时才可知）。
    Dynamic,
}

impl IrEffect {
    /// 从宿主效果种类映射到 MIR 效果（`AudioEmit` 归入非确定性）。
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

/// MIR 值引用（SSA / 临时编号，函数内唯一）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirValue(pub u32);

/// 基本块终结。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirTerminator {
    /// 返回；`None` 表示无返回值。
    Return {
        /// 可选返回值。
        value: Option<MirValue>,
    },
    /// 无条件跳转到目标块。
    Jump {
        /// 目标基本块 id。
        target: u32,
    },
    /// 条件分支。
    Branch {
        /// 条件值（真假语义由 lowering 约定）。
        cond: MirValue,
        /// 真分支目标块。
        then_target: u32,
        /// 假分支目标块。
        else_target: u32,
    },
    /// 不可达（优化占位或错误路径）。
    Unreachable,
}

/// MIR 指令（非终结）。
#[derive(Debug, Clone, PartialEq)]
pub enum MirInst {
    /// 空操作。
    Nop,
    /// 写入 null 常量。
    ConstNull {
        /// 目标临时。
        dst: MirValue,
    },
    /// 写入布尔常量。
    ConstBool {
        /// 目标临时。
        dst: MirValue,
        /// 常量值。
        value: bool,
    },
    /// 写入数值常量（f64 盒）。
    ConstNumber {
        /// 目标临时。
        dst: MirValue,
        /// 常量值。
        value: f64,
    },
    /// 写入字符串常量（入池由 codegen 处理）。
    ConstString {
        /// 目标临时。
        dst: MirValue,
        /// 字符串正文。
        value: Arc<str>,
    },
    /// 写入函数引用常量。
    ConstFunc {
        /// 目标临时。
        dst: MirValue,
        /// 模块内函数下标。
        func_index: u32,
    },
    /// 读取局部。
    LoadLocal {
        /// 目标临时。
        dst: MirValue,
        /// 局部槽位。
        index: u32,
    },
    /// 写入局部。
    StoreLocal {
        /// 局部槽位。
        index: u32,
        /// 源临时。
        src: MirValue,
    },
    /// 临时间拷贝。
    Move {
        /// 目标。
        dst: MirValue,
        /// 源。
        src: MirValue,
    },
    /// 二元运算。
    Binary {
        /// 结果。
        dst: MirValue,
        /// 运算符。
        op: HirBinaryOp,
        /// 左操作数。
        lhs: MirValue,
        /// 右操作数。
        rhs: MirValue,
    },
    /// 一元运算。
    Unary {
        /// 结果。
        dst: MirValue,
        /// 运算符。
        op: HirUnaryOp,
        /// 操作数。
        src: MirValue,
    },
    /// 调用脚本函数（`func` 为函数值临时）。
    Call {
        /// 可选返回值写入目标。
        dst: Option<MirValue>,
        /// 被调函数值。
        func: MirValue,
        /// 实参列表。
        args: Vec<MirValue>,
    },
    /// 调试打印。
    Print {
        /// 打印源值。
        src: MirValue,
    },
    /// 调用宿主。
    HostCall {
        /// 可选返回值写入目标。
        dst: Option<MirValue>,
        /// 宿主身份或已解析槽位。
        host_slot_or_name: HostRef,
        /// 实参列表。
        args: Vec<MirValue>,
    },
    /// 动态方法发送（接收者 + 方法名）。
    DynamicSend {
        /// 可选返回值写入目标。
        dst: Option<MirValue>,
        /// 接收者对象。
        receiver: MirValue,
        /// 方法名。
        method: Arc<str>,
        /// 实参列表。
        args: Vec<MirValue>,
    },
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
    /// 块 id（函数内唯一）。
    pub id: u32,
    /// 非终结指令序列。
    pub insts: Vec<MirInst>,
    /// 块终结器（必须恰好一个）。
    pub terminator: MirTerminator,
}

/// MIR 函数。
#[derive(Debug, Clone, PartialEq)]
pub struct MirFunction {
    /// 函数名。
    pub name: Arc<str>,
    /// 形参个数。
    pub arity: u8,
    /// 局部类型表（含形参）。
    pub local_tys: Vec<Ty>,
    /// 基本块列表（入口一般为 id 0）。
    pub blocks: Vec<BasicBlock>,
    /// 函数效果汇总。
    pub effects: Vec<IrEffect>,
}

/// MIR 模块。
#[derive(Debug, Clone, PartialEq)]
pub struct MirModule {
    /// 所属包 id。
    pub package: PackageId,
    /// 模块名。
    pub name: Arc<str>,
    /// 函数列表。
    pub functions: Vec<MirFunction>,
}
