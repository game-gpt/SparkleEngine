//! 脚本运行域：每个模组包对应一个隔离实例。
//!
//! [`ScriptDomain`] 持有已验证映像的运行时、宿主 schema 指纹、生命周期导出与预算。
//! **不**拥有 ECS [`spark_ecs`] 世界；结构变更须经后续命令缓冲在同步点提交。

use std::{cell::RefCell, rc::Rc, sync::Arc};

use spark_gc::Value;
use spark_script::{ExecutableImage, HostPhase, HostSchema, ScriptRuntime};
use spark_vm::HostHooks;

use crate::{EngineError, command_buffer::ScriptCommandBuffer, event_inbox::ScriptEventInbox};

/// 每领域每帧（或每次回调）的资源预算。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBudget {
    /// VM 指令步上限（写入 `vm.step_limit`）。
    pub instruction_limit: u64,
    /// 宿主导入调用次数上限。
    pub host_call_limit: u64,
    /// 分配字节上限（由 VM 解释）。
    pub allocation_limit: u64,
    /// 调用栈深度上限。
    pub call_depth_limit: u16,
}

impl Default for ScriptBudget {
    fn default() -> Self {
        Self { instruction_limit: 5_000_000, host_call_limit: 100_000, allocation_limit: 1_000_000, call_depth_limit: 256 }
    }
}

/// 模组脚本运行域（串行资源：同域执行暂不并行）。
pub struct ScriptDomain {
    /// 所属模组 id。
    pub mod_id: Arc<str>,
    /// 已装载映像的运行时（含 VM）。
    pub runtime: ScriptRuntime,
    /// 装载时宿主 schema 指纹（与映像校验一致）。
    pub host_schema_hash: u64,
    /// 宿主 ABI 版本号。
    pub host_abi_version: u32,
    /// 映像声明的生命周期导出名列表。
    pub lifecycle_exports: Vec<Arc<str>>,
    /// 当前资源预算（可在运行中调整后 `apply_budget`）。
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
    pub fn call(&mut self, name: &str, args: &[Value], host: &mut dyn HostHooks) -> Result<Value, EngineError> {
        self.call_in_phase(name, args, HostPhase::Any, host)
    }

    /// 在指定 [`HostPhase`] 下调用导出（调度器须先在 [`EngineShared`] 写入阶段与访问策略）。
    pub fn call_in_phase(&mut self, name: &str, args: &[Value], phase: HostPhase, host: &mut dyn HostHooks) -> Result<Value, EngineError> {
        if !self.enabled {
            return Err(EngineError::ScriptDomainDisabled { mod_id: self.mod_id.to_string() });
        }
        // 阶段门禁由引擎在 `EngineShared.active_phase` + 内置原生 `gate` 强制；
        // 此处保留参数供调用方审计与未来 VM 级槽位检查。
        let _ = phase;
        self.runtime.call(name, args, host).map_err(EngineError::Script)
    }

    /// 若存在则调用生命周期导出；不存在则返回 `None`。
    pub fn call_lifecycle(&mut self, name: &str, args: &[Value], host: &mut dyn HostHooks) -> Result<Option<Value>, EngineError> {
        if !self.has_lifecycle(name) {
            return Ok(None);
        }
        self.call(name, args, host).map(Some)
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
    pub fn dispatch_events(&mut self, host: &mut dyn HostHooks) -> Result<(), EngineError> {
        let events = self.drain_events();
        for ev in events {
            if self.has_lifecycle("on_event") {
                let _ = self.call("on_event", &ev.args, host)?;
            }
            else if self.runtime.vm.module.functions.iter().any(|f| f.name == ev.name.as_ref()) {
                let _ = self.call(ev.name.as_ref(), &ev.args, host)?;
            }
        }
        Ok(())
    }
}
