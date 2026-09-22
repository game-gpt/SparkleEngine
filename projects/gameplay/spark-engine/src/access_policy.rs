//! 脚本 System 调用期的宿主访问策略。
//!
//! 调度器在每次 `call_in_phase` 前写入 [`crate::EngineShared`]；内置原生据此强制
//! 阶段、确定性、组件写集、世界只读与原型可见集，避免描述符沦为文档元数据。

use std::{collections::HashSet, sync::Arc};

use spark_script::{DeterminismClass, HostPhase, HostSchema};

use crate::script_system::ScriptSystemDescriptor;

/// 当前脚本调用的组件 / 查询访问策略。
#[derive(Debug, Clone, Default)]
pub enum ScriptAccessPolicy {
    /// 未挂 System 描述符（钩子 / 装载回调）：不强制访问集。
    #[default]
    Unrestricted,
    /// 已挂描述符：写集约束组件变更；读世界须声明至少一项 read 或 write。
    /// `archetypes` 非空时过滤 `query_*` 可见原型。
    Declared { reads: HashSet<Arc<str>>, writes: HashSet<Arc<str>>, archetypes: HashSet<Arc<str>> },
}

impl ScriptAccessPolicy {
    pub fn from_descriptor(desc: &ScriptSystemDescriptor) -> Self {
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();
        for a in &desc.access {
            if a.write {
                writes.insert(Arc::clone(&a.component));
            }
            else {
                reads.insert(Arc::clone(&a.component));
            }
        }
        let archetypes = desc.query_archetypes.iter().cloned().collect();
        Self::Declared { reads, writes, archetypes }
    }

    /// 是否允许对 `component` 做写变更（`queue_add_component` 等）。
    pub fn allows_write(&self, component: &str) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::Declared { writes, .. } => writes.iter().any(|c| c.as_ref() == component),
        }
    }

    /// 是否允许只读查询世界（`query_*`）。
    ///
    /// `Declared` 且读写集皆空 → 拒绝（未声明任何世界访问）。
    pub fn allows_read_world(&self) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::Declared { reads, writes, .. } => !reads.is_empty() || !writes.is_empty(),
        }
    }

    /// 原型过滤集；`None` = 不按原型过滤。
    pub fn archetype_filter(&self) -> Option<&HashSet<Arc<str>>> {
        match self {
            Self::Unrestricted => None,
            Self::Declared { archetypes, .. } if archetypes.is_empty() => None,
            Self::Declared { archetypes, .. } => Some(archetypes),
        }
    }

    /// 是否允许查询该原型名。
    pub fn allows_query_archetype(&self, archetype: &str) -> bool {
        match self.archetype_filter() {
            None => true,
            Some(allow) => allow.iter().any(|a| a.as_ref() == archetype),
        }
    }
}

/// 按 schema + 当前阶段检查宿主导入名是否可调用（限定名或唯一短名）。
pub fn check_host_phase(schema: &HostSchema, import: &str, phase: HostPhase) -> Result<(), String> {
    let func = schema.resolve_import(import)?;
    if func.allows_phase(phase) { Ok(()) } else { Err(format!("host_phase_denied:{import}:phase={phase:?}")) }
}

/// System 确定性要求是否允许调用该宿主导入名。
pub fn check_host_determinism(schema: &HostSchema, import: &str, required: DeterminismClass) -> Result<(), String> {
    let func = schema.resolve_import(import)?;
    if determinism_allows(required, func.determinism) {
        Ok(())
    }
    else {
        Err(format!("host_determinism_denied:{import}:policy={required:?}:host={:?}", func.determinism))
    }
}

fn determinism_allows(policy: DeterminismClass, host: DeterminismClass) -> bool {
    match policy {
        DeterminismClass::Nondeterministic => true,
        DeterminismClass::FrameLocal => !matches!(host, DeterminismClass::Nondeterministic),
        DeterminismClass::Deterministic => matches!(host, DeterminismClass::Deterministic),
    }
}
