//! Rust System 元数据：进入统一调度图前的声明（与脚本 [`ScriptSystemDescriptor`] 对称）。

use std::any::TypeId;

use super::phase::RustPhase;

/// Rust 原生 System 调度声明。
#[derive(Debug, Clone)]
pub struct RustSystemMeta {
    /// 逻辑名（同相位内唯一；调度图键 `rust/{name}`）。
    pub name: &'static str,
    /// 所属 Rust 相位。
    pub phase: RustPhase,
    /// 须排在目标 **之前**（可写 `rust/foo` 或脚本 `{mod}/{name}`）。
    pub before: Vec<&'static str>,
    /// 须排在目标 **之后**。
    pub after: Vec<&'static str>,
    /// 只读触及的组件 / 资源 [`TypeId`]（并行预留）。
    pub reads: Vec<TypeId>,
    /// 可写触及的组件 / 资源 [`TypeId`]（并行预留）。
    pub writes: Vec<TypeId>,
}

impl RustSystemMeta {
    /// 最小声明：无顺序与访问约束。
    pub fn new(name: &'static str, phase: RustPhase) -> Self {
        Self { name, phase, before: Vec::new(), after: Vec::new(), reads: Vec::new(), writes: Vec::new() }
    }

    /// 调度图节点键。
    pub fn graph_key(&self) -> String {
        format!("rust/{}", self.name)
    }
}
