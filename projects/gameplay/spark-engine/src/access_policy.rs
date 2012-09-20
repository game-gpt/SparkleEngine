//! 脚本 System 调用期的宿主访问策略。
//!
//! 调度器在每次 `call_in_phase` 前写入 [`crate::EngineShared`]；内置原生据此强制
//! 阶段、确定性、组件写集与世界只读查询，避免描述符沦为文档元数据。

use std::collections::HashSet;
use std::sync::Arc;

use spark_script::{DeterminismClass, HostPhase, HostSchema};

use crate::script_system::ScriptSystemDescriptor;

/// 当前脚本调用的组件访问策略。
#[derive(Debug, Clone, Default)]
pub enum ScriptAccessPolicy {
    /// 未挂 System 描述符（钩子 / 装载回调）：不强制访问集。
    #[default]
    Unrestricted,
    /// 已挂描述符：写集约束组件变更；读世界须声明至少一项 read 或 write。
    Declared {
        reads: HashSet<Arc<str>>,
        writes: HashSet<Arc<str>>,
    },
}

impl ScriptAccessPolicy {
    pub fn from_descriptor(desc: &ScriptSystemDescriptor) -> Self {
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();
        for a in &desc.access {
            if a.write {
                writes.insert(Arc::clone(&a.component));
            } else {
                reads.insert(Arc::clone(&a.component));
            }
        }
        Self::Declared { reads, writes }
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
            Self::Declared { reads, writes } => !reads.is_empty() || !writes.is_empty(),
        }
    }
}

/// 按 schema + 当前阶段检查宿主短名是否可调用。
pub fn check_host_phase(
    schema: &HostSchema,
    short_name: &str,
    phase: HostPhase,
) -> Result<(), String> {
    match schema.resolve_short_name(short_name) {
        Ok(func) => {
            if func.allows_phase(phase) {
                Ok(())
            } else {
                Err(format!(
                    "host_phase_denied:{short_name}:phase={phase:?}"
                ))
            }
        }
        Err(e) if e.starts_with("host_unknown:") => Ok(()),
        Err(e) => Err(e),
    }
}

/// System 确定性要求是否允许调用该宿主短名。
pub fn check_host_determinism(
    schema: &HostSchema,
    short_name: &str,
    required: DeterminismClass,
) -> Result<(), String> {
    match schema.resolve_short_name(short_name) {
        Ok(func) => {
            if determinism_allows(required, func.determinism) {
                Ok(())
            } else {
                Err(format!(
                    "host_determinism_denied:{short_name}:policy={required:?}:host={:?}",
                    func.determinism
                ))
            }
        }
        Err(e) if e.starts_with("host_unknown:") => Ok(()),
        Err(e) => Err(e),
    }
}

fn determinism_allows(policy: DeterminismClass, host: DeterminismClass) -> bool {
    match policy {
        DeterminismClass::Nondeterministic => true,
        DeterminismClass::FrameLocal => !matches!(host, DeterminismClass::Nondeterministic),
        DeterminismClass::Deterministic => matches!(host, DeterminismClass::Deterministic),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_script::{HostEffect, HostFunction, HostFunctionId};

    #[test]
    fn declared_empty_write_set_blocks_component_mutation() {
        let desc = ScriptSystemDescriptor::new("m", "s", "update", HostPhase::Update)
            .read("Transform");
        let policy = ScriptAccessPolicy::from_descriptor(&desc);
        assert!(!policy.allows_write("Transform"));
        assert!(!policy.allows_write("Health"));
        assert!(policy.allows_read_world());
    }

    #[test]
    fn declared_empty_access_blocks_world_query() {
        let desc = ScriptSystemDescriptor::new("m", "s", "update", HostPhase::Update);
        let policy = ScriptAccessPolicy::from_descriptor(&desc);
        assert!(!policy.allows_read_world());
    }

    #[test]
    fn check_host_phase_denies_wrong_phase() {
        let mut schema = HostSchema::new(1);
        schema.insert(
            HostFunction::new(HostFunctionId::new("engine", "queue_spawn", 1))
                .phases([HostPhase::Update])
                .effect(HostEffect::SpawnEntity),
        );
        assert!(check_host_phase(&schema, "queue_spawn", HostPhase::Update).is_ok());
        let err = check_host_phase(&schema, "queue_spawn", HostPhase::RenderPrepare).unwrap_err();
        assert!(err.contains("host_phase_denied"), "{err}");
    }

    #[test]
    fn check_host_determinism_denies_nondeterministic() {
        let mut schema = HostSchema::new(1);
        schema.insert(
            HostFunction::new(HostFunctionId::new("engine", "log", 1))
                .determinism(DeterminismClass::Nondeterministic),
        );
        let err = check_host_determinism(
            &schema,
            "log",
            DeterminismClass::Deterministic,
        )
        .unwrap_err();
        assert!(err.contains("host_determinism_denied"), "{err}");
    }
}
