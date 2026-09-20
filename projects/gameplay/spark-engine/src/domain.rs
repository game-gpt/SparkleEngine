//! 脚本运行域：每个模组包对应一个隔离实例。
//!
//! [`ScriptDomain`] 持有已验证映像的运行时、宿主 schema 指纹、生命周期导出与预算。
//! **不**拥有 ECS [`spark_ecs`] 世界；结构变更须经后续命令缓冲在同步点提交。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use spark_gc::Value;
use spark_script::{ExecutableImage, HostSchema, ScriptLanguage, ScriptRuntime};
use spark_vm::{HostHooks, Module};

use crate::command_buffer::ScriptCommandBuffer;
use crate::event_inbox::ScriptEventInbox;
use crate::EngineError;

/// 每领域每帧（或每次回调）的资源预算。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBudget {
    pub instruction_limit: u64,
    pub host_call_limit: u64,
    pub allocation_limit: u64,
    pub call_depth_limit: u16,
}

impl Default for ScriptBudget {
    fn default() -> Self {
        Self {
            instruction_limit: 5_000_000,
            host_call_limit: 100_000,
            allocation_limit: 1_000_000,
            call_depth_limit: 256,
        }
    }
}

/// 模组脚本运行域（串行资源：同域执行暂不并行）。
pub struct ScriptDomain {
    pub mod_id: Arc<str>,
    pub runtime: ScriptRuntime,
    pub host_schema_hash: u64,
    pub host_abi_version: u32,
    pub lifecycle_exports: Vec<Arc<str>>,
    pub budget: ScriptBudget,
    /// 结构变更意图，同步点由引擎 `drain` 后提交。
    /// 与宿主 `queue_*` 原生共享同一缓冲。
    pub command_buffer: Rc<RefCell<ScriptCommandBuffer>>,
    /// 待派发事件（禁止同步回调嵌套重入）。
    pub event_inbox: ScriptEventInbox,
    /// 领域是否仍可被调度（trap / 预算耗尽后可置 false）。
    pub enabled: bool,
}

impl ScriptDomain {
    /// 从已验证映像与同一份 [`HostSchema`] 创建领域。
    pub fn from_image(
        mod_id: impl Into<Arc<str>>,
        image: &ExecutableImage,
        host: &HostSchema,
        budget: ScriptBudget,
    ) -> Result<Self, EngineError> {
        let runtime = ScriptRuntime::from_image(image, host).map_err(EngineError::Script)?;
        let mut domain = Self {
            mod_id: mod_id.into(),
            host_schema_hash: runtime.host_schema_hash(),
            host_abi_version: runtime.host_abi_version(),
            lifecycle_exports: image.lifecycle_exports.clone(),
            runtime,
            budget: budget.clone(),
            command_buffer: Rc::new(RefCell::new(ScriptCommandBuffer::new())),
            event_inbox: ScriptEventInbox::new(),
            enabled: true,
        };
        domain.apply_budget();
        Ok(domain)
    }

    /// 是否声明了某一生命周期导出。
    pub fn has_lifecycle(&self, name: &str) -> bool {
        self.lifecycle_exports.iter().any(|n| n.as_ref() == name)
    }

    /// 将领域预算同步到 VM。
    pub fn apply_budget(&mut self) {
        self.runtime.vm.step_limit = self.budget.instruction_limit;
        self.runtime.vm.host_call_limit = self.budget.host_call_limit;
        self.runtime.vm.allocation_limit = self.budget.allocation_limit;
        self.runtime.vm.call_depth_limit = self.budget.call_depth_limit;
    }

    /// 调用命名导出（生命周期或普通函数）。
    pub fn call(
        &mut self,
        name: &str,
        args: &[Value],
        host: &mut dyn HostHooks,
    ) -> Result<Value, EngineError> {
        if !self.enabled {
            return Err(EngineError::ScriptDomainDisabled {
                mod_id: self.mod_id.to_string(),
            });
        }
        self.runtime
            .call(name, args, host)
            .map_err(EngineError::Script)
    }

    /// 若存在则调用生命周期导出；不存在则返回 `None`（不跑隐式 `__main`）。
    pub fn call_lifecycle(
        &mut self,
        name: &str,
        args: &[Value],
        host: &mut dyn HostHooks,
    ) -> Result<Option<Value>, EngineError> {
        if !self.has_lifecycle(name) {
            return Ok(None);
        }
        self.call(name, args, host).map(Some)
    }

    /// 过渡期：执行映像入口（旧 `__main` / 顶层）。新模组应改用生命周期导出。
    pub fn eval_entry(&mut self, host: &mut dyn HostHooks) -> Result<Value, EngineError> {
        if !self.enabled {
            return Err(EngineError::ScriptDomainDisabled {
                mod_id: self.mod_id.to_string(),
            });
        }
        self.runtime.eval_with(host).map_err(EngineError::Script)
    }

    /// 过渡期：从裸 [`Module`] 创建领域（跳过制品校验，供测试与旧路径）。
    pub fn from_legacy_module(
        mod_id: impl Into<Arc<str>>,
        module: Module,
        language: ScriptLanguage,
        budget: ScriptBudget,
    ) -> Self {
        let lifecycle_exports = module
            .functions
            .iter()
            .filter(|f| is_lifecycle_name(&f.name))
            .map(|f| Arc::<str>::from(f.name.as_str()))
            .collect();
        let runtime = ScriptRuntime::from_legacy_module(module, language);
        let mut domain = Self {
            mod_id: mod_id.into(),
            host_schema_hash: 0,
            host_abi_version: 0,
            lifecycle_exports,
            runtime,
            budget: budget.clone(),
            command_buffer: Rc::new(RefCell::new(ScriptCommandBuffer::new())),
            event_inbox: ScriptEventInbox::new(),
            enabled: true,
        };
        domain.apply_budget();
        domain
    }

    /// 取出并清空本领域命令缓冲（帧同步点调用）。
    pub fn drain_commands(&mut self) -> Vec<crate::ScriptCommand> {
        self.command_buffer.borrow_mut().drain()
    }

    /// 取出并清空事件 inbox（在允许的 phase 批量派发前调用）。
    pub fn drain_events(&mut self) -> Vec<crate::ScriptEvent> {
        self.event_inbox.drain()
    }

    /// 将事件入队（不立即回调脚本）。
    pub fn enqueue_event(&mut self, name: impl Into<Arc<str>>, args: Vec<Value>) {
        self.event_inbox.push(name, args);
    }

    /// 派发 inbox 中全部事件到 `on_event`（若导出）或同名导出函数。
    pub fn dispatch_events(
        &mut self,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        let events = self.drain_events();
        for ev in events {
            if self.has_lifecycle("on_event") {
                let _ = self.call("on_event", &ev.args, host)?;
            } else if self
                .runtime
                .vm
                .module
                .functions
                .iter()
                .any(|f| f.name == ev.name.as_ref())
            {
                let _ = self.call(ev.name.as_ref(), &ev.args, host)?;
            }
        }
        Ok(())
    }
}

fn is_lifecycle_name(name: &str) -> bool {
    matches!(
        name,
        "on_load"
            | "on_start"
            | "fixed_update"
            | "update"
            | "late_update"
            | "render_prepare"
            | "on_event"
            | "on_unload"
            | "save_state"
            | "load_state"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_script::{
        HostFunction, HostFunctionId, ScriptCompiler, ScriptLanguage,
    };
    use spark_vm::StdHost;

    #[test]
    fn domain_loads_image_and_calls_lifecycle() {
        let mut host = HostSchema::new(1);
        host.insert(HostFunction::new(HostFunctionId::new("host", "noop", 1)));
        let source = r#"
            micro on_start() {
                return 42
            }
            return 0
            "#;
        let mut compiler = ScriptCompiler::new();
        let package = compiler
            .compile_source(ScriptLanguage::Valkyrie, source, &host)
            .unwrap();
        assert!(package
            .image
            .lifecycle_exports
            .iter()
            .any(|n| n.as_ref() == "on_start"));
        let mut domain =
            ScriptDomain::from_image("test.mod", &package.image, &host, ScriptBudget::default())
                .unwrap();
        let mut hooks = StdHost;
        let v = domain
            .call_lifecycle("on_start", &[], &mut hooks)
            .unwrap()
            .unwrap();
        assert_eq!(v.as_number(), Some(42.0));
        assert!(domain
            .call_lifecycle("on_unload", &[], &mut hooks)
            .unwrap()
            .is_none());
        assert_eq!(
            domain.runtime.vm.step_limit,
            ScriptBudget::default().instruction_limit
        );
    }

    #[test]
    fn custom_budget_sets_vm_limits() {
        let host = HostSchema::new(1);
        let mut compiler = ScriptCompiler::new();
        let package = compiler
            .compile_source(ScriptLanguage::Valkyrie, "return 1", &host)
            .unwrap();
        let budget = ScriptBudget {
            instruction_limit: 1234,
            host_call_limit: 56,
            allocation_limit: 78,
            call_depth_limit: 9,
        };
        let domain =
            ScriptDomain::from_image("budget.mod", &package.image, &host, budget).unwrap();
        assert_eq!(domain.runtime.vm.step_limit, 1234);
        assert_eq!(domain.runtime.vm.host_call_limit, 56);
        assert_eq!(domain.runtime.vm.allocation_limit, 78);
        assert_eq!(domain.runtime.vm.call_depth_limit, 9);
    }

    #[test]
    fn domain_command_buffer_drains() {
        let host = HostSchema::new(1);
        let mut compiler = ScriptCompiler::new();
        let package = compiler
            .compile_source(ScriptLanguage::Valkyrie, "return 1", &host)
            .unwrap();
        let mut domain =
            ScriptDomain::from_image("buf.mod", &package.image, &host, ScriptBudget::default())
                .unwrap();
        domain.command_buffer.borrow_mut().spawn("rock");
        let cmds = domain.drain_commands();
        assert_eq!(cmds.len(), 1);
        assert!(domain.command_buffer.borrow().is_empty());
    }

    #[test]
    fn domain_dispatches_named_event_export() {
        let source = r#"
            micro ping() {
                return 7
            }
            return 0
            "#;
        let host = HostSchema::new(1);
        let mut compiler = ScriptCompiler::new();
        let package = compiler
            .compile_source(ScriptLanguage::Valkyrie, source, &host)
            .unwrap();
        let mut domain =
            ScriptDomain::from_image("ev.mod", &package.image, &host, ScriptBudget::default())
                .unwrap();
        domain.enqueue_event("ping", vec![]);
        let mut hooks = StdHost;
        domain.dispatch_events(&mut hooks).unwrap();
        assert!(domain.event_inbox.is_empty());
    }
}