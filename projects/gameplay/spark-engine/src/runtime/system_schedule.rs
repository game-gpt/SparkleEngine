//! 原生 System 调度声明（进入统一调度图前）；与 [`ScriptSystemDescriptor`] 对称，**非**独立「RustSystem」产品类型。

use std::any::TypeId;

use super::phase::RustPhase;

/// 作者可写的顺序与组件访问约束（与脚本 [`ComponentAccess`] 同名对齐，供混排冲突检测）。
#[derive(Debug, Clone, Default)]
pub struct SystemOrder {
    /// 须排在目标 **之前**（可写 `native/foo` 或脚本 `{mod}/{name}`）。
    pub before: Vec<&'static str>,
    /// 须排在目标 **之后**。
    pub after: Vec<&'static str>,
    /// 只读触及的组件逻辑名（与脚本 System 声明同一命名空间）。
    pub reads: Vec<&'static str>,
    /// 可写触及的组件逻辑名。
    pub writes: Vec<&'static str>,
}

/// 调度图登记用的原生 System 元数据（crate 内部）。
#[derive(Debug, Clone)]
pub(crate) struct SystemSchedule {
    /// 逻辑名（同相位内唯一；调度图键 `native/{name}`）。
    pub name: &'static str,
    /// 所属 Rust 相位。
    pub phase: RustPhase,
    /// 须排在目标 **之前**。
    pub before: Vec<&'static str>,
    /// 须排在目标 **之后**。
    pub after: Vec<&'static str>,
    /// 只读触及的组件 / 资源 [`TypeId`]（并行预留；Rust 域内冲突检测）。
    pub reads: Vec<TypeId>,
    /// 可写触及的组件 / 资源 [`TypeId`]（并行预留；Rust 域内冲突检测）。
    pub writes: Vec<TypeId>,
    /// 只读组件逻辑名（跨域与脚本 [`ComponentAccess`] 冲突检测）。
    pub read_components: Vec<&'static str>,
    /// 可写组件逻辑名。
    pub write_components: Vec<&'static str>,
}

impl SystemSchedule {
    /// 最小声明：无顺序与访问约束。
    pub fn new(name: &'static str, phase: RustPhase) -> Self {
        Self {
            name,
            phase,
            before: Vec::new(),
            after: Vec::new(),
            reads: Vec::new(),
            writes: Vec::new(),
            read_components: Vec::new(),
            write_components: Vec::new(),
        }
    }

    /// 从 [`SystemOrder`] 构造。
    pub fn with_order(name: &'static str, phase: RustPhase, order: &SystemOrder) -> Self {
        Self {
            name,
            phase,
            before: order.before.clone(),
            after: order.after.clone(),
            reads: Vec::new(),
            writes: Vec::new(),
            read_components: order.reads.clone(),
            write_components: order.writes.clone(),
        }
    }

    /// 调度图节点键。
    pub fn graph_key(&self) -> String {
        format!("native/{}", self.name)
    }
}
