//! 脚本 System 描述符：进入与 Rust System 同一调度图前的声明契约。
//!
//! 初版只登记元数据；真正的查询游标与并行调度后续接入。
//! 同一 [`crate::ScriptDomain`] 的执行默认视为串行资源。

use std::sync::Arc;

use spark_script::{DeterminismClass, HostPhase};

/// 脚本 System 的并行策略（ECS 读写集不足以证明可并行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScriptParallelism {
    /// 与同域其它脚本串行（默认）。
    #[default]
    SerialDomain,
    /// 与一切脚本 System 互斥。
    Exclusive,
}

/// 组件访问声明（名字在链接期解析为稳定槽位；热路径禁止再字符串查找）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentAccess {
    pub component: Arc<str>,
    pub write: bool,
}

impl ComponentAccess {
    pub fn read(component: impl Into<Arc<str>>) -> Self {
        Self {
            component: component.into(),
            write: false,
        }
    }

    pub fn write(component: impl Into<Arc<str>>) -> Self {
        Self {
            component: component.into(),
            write: true,
        }
    }
}

/// 脚本 System 描述符。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSystemDescriptor {
    pub mod_id: Arc<str>,
    pub name: Arc<str>,
    /// 入口导出函数名（如 `fixed_update` / 自定义 micro）。
    pub entry: Arc<str>,
    pub phase: HostPhase,
    pub access: Vec<ComponentAccess>,
    pub before: Vec<Arc<str>>,
    pub after: Vec<Arc<str>>,
    pub determinism: DeterminismClass,
    pub parallelism: ScriptParallelism,
}

impl ScriptSystemDescriptor {
    pub fn new(
        mod_id: impl Into<Arc<str>>,
        name: impl Into<Arc<str>>,
        entry: impl Into<Arc<str>>,
        phase: HostPhase,
    ) -> Self {
        Self {
            mod_id: mod_id.into(),
            name: name.into(),
            entry: entry.into(),
            phase,
            access: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            determinism: DeterminismClass::Deterministic,
            parallelism: ScriptParallelism::SerialDomain,
        }
    }

    pub fn read(mut self, component: impl Into<Arc<str>>) -> Self {
        self.access.push(ComponentAccess::read(component));
        self
    }

    pub fn write(mut self, component: impl Into<Arc<str>>) -> Self {
        self.access.push(ComponentAccess::write(component));
        self
    }

    pub fn before(mut self, name: impl Into<Arc<str>>) -> Self {
        self.before.push(name.into());
        self
    }

    pub fn after(mut self, name: impl Into<Arc<str>>) -> Self {
        self.after.push(name.into());
        self
    }

    pub fn determinism(mut self, class: DeterminismClass) -> Self {
        self.determinism = class;
        self
    }

    pub fn parallelism(mut self, policy: ScriptParallelism) -> Self {
        self.parallelism = policy;
        self
    }
}

/// 引擎侧脚本 System 登记表（按插入顺序；同域串行由调度器保证）。
#[derive(Debug, Default, Clone)]
pub struct ScriptSystemRegistry {
    systems: Vec<ScriptSystemDescriptor>,
}

impl ScriptSystemRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    pub fn register(&mut self, desc: ScriptSystemDescriptor) {
        if let Some(existing) = self
            .systems
            .iter_mut()
            .find(|s| s.mod_id == desc.mod_id && s.name == desc.name)
        {
            *existing = desc;
        } else {
            self.systems.push(desc);
        }
    }

    pub fn remove_mod(&mut self, mod_id: &str) {
        self.systems.retain(|s| s.mod_id.as_ref() != mod_id);
    }

    pub fn systems(&self) -> &[ScriptSystemDescriptor] {
        &self.systems
    }

    pub fn for_phase(&self, phase: HostPhase) -> impl Iterator<Item = &ScriptSystemDescriptor> {
        self.systems
            .iter()
            .filter(move |s| s.phase == phase || s.phase == HostPhase::Any)
    }

    /// 由领域生命周期导出生成默认 System（每导出一条）。
    pub fn register_lifecycle_exports(
        &mut self,
        mod_id: impl Into<Arc<str>>,
        exports: &[Arc<str>],
    ) {
        let mod_id = mod_id.into();
        for name in exports {
            let Some(phase) = lifecycle_phase(name.as_ref()) else {
                continue;
            };
            self.register(ScriptSystemDescriptor::new(
                Arc::clone(&mod_id),
                Arc::clone(name),
                Arc::clone(name),
                phase,
            ));
        }
    }
}

fn lifecycle_phase(name: &str) -> Option<HostPhase> {
    match name {
        "on_load" => Some(HostPhase::OnLoad),
        "on_start" => Some(HostPhase::OnStart),
        "fixed_update" => Some(HostPhase::FixedUpdate),
        "update" => Some(HostPhase::Update),
        "late_update" => Some(HostPhase::LateUpdate),
        "render_prepare" => Some(HostPhase::RenderPrepare),
        "on_event" => Some(HostPhase::OnEvent),
        "on_unload" => Some(HostPhase::OnUnload),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_filters_by_phase() {
        let mut reg = ScriptSystemRegistry::new();
        reg.register(ScriptSystemDescriptor::new(
            "demo",
            "move",
            "fixed_update",
            HostPhase::FixedUpdate,
        ));
        reg.register(
            ScriptSystemDescriptor::new("demo", "draw", "render_prepare", HostPhase::RenderPrepare)
                .read("Transform"),
        );
        assert_eq!(reg.for_phase(HostPhase::FixedUpdate).count(), 1);
        assert_eq!(reg.for_phase(HostPhase::RenderPrepare).count(), 1);
        assert_eq!(reg.for_phase(HostPhase::Update).count(), 0);
    }

    #[test]
    fn lifecycle_exports_register_phases() {
        let mut reg = ScriptSystemRegistry::new();
        let exports = [
            Arc::<str>::from("on_load"),
            Arc::<str>::from("update"),
            Arc::<str>::from("helper"),
        ];
        reg.register_lifecycle_exports("m", &exports);
        assert_eq!(reg.len(), 2);
        assert!(reg
            .for_phase(HostPhase::Update)
            .any(|s| s.entry.as_ref() == "update"));
    }
}
