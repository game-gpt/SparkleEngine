//! 脚本命令缓冲：结构变更延迟到同步点提交。
//!
//! 脚本不得在查询遍历中直接改 ECS 结构。意图写入本缓冲，由 [`spark-engine`]
//! 在帧同步点合并提交到世界。

use std::sync::Arc;

/// 延迟结构变更意图（初版枚举，后续对接真实 ECS 描述符槽）。
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptCommand {
    /// 按原型名生成实体（原型解析在提交期完成）。
    Spawn {
        /// 脚本原型名；提交期挂 [`crate::ScriptArchetypeTag`]。
        archetype: Arc<str>,
    },
    /// 销毁实体（代际句柄，提交期校验）。
    Despawn {
        /// 实体位模式（`Entity::to_bits`）。
        entity: u64,
    },
    /// 添加组件（组件槽在链接期解析；此处暂用名字）。
    AddComponent {
        /// 目标实体位模式。
        entity: u64,
        /// 组件逻辑名（须在组件目录中登记且可应用）。
        component: Arc<str>,
    },
    /// 移除组件。
    RemoveComponent {
        /// 目标实体位模式。
        entity: u64,
        /// 组件逻辑名。
        component: Arc<str>,
    },
}

/// 每领域一份命令缓冲。同步点 [`Self::drain`] 后交给引擎提交。
#[derive(Debug, Default, Clone)]
pub struct ScriptCommandBuffer {
    commands: Vec<ScriptCommand>,
}

impl ScriptCommandBuffer {
    /// 空缓冲。
    pub fn new() -> Self {
        Self::default()
    }

    /// 排队命令条数。
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// 是否无排队命令。
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// 追加任意命令。
    pub fn push(&mut self, cmd: ScriptCommand) {
        self.commands.push(cmd);
    }

    /// 排队生成带指定原型名的实体。
    pub fn spawn(&mut self, archetype: impl Into<Arc<str>>) {
        self.push(ScriptCommand::Spawn { archetype: archetype.into() });
    }

    /// 排队销毁实体。
    pub fn despawn(&mut self, entity: u64) {
        self.push(ScriptCommand::Despawn { entity });
    }

    /// 排队向实体添加组件。
    pub fn add_component(&mut self, entity: u64, component: impl Into<Arc<str>>) {
        self.push(ScriptCommand::AddComponent { entity, component: component.into() });
    }

    /// 排队从实体移除组件。
    pub fn remove_component(&mut self, entity: u64, component: impl Into<Arc<str>>) {
        self.push(ScriptCommand::RemoveComponent { entity, component: component.into() });
    }

    /// 取出全部命令并清空缓冲（供同步点提交）。
    pub fn drain(&mut self) -> Vec<ScriptCommand> {
        std::mem::take(&mut self.commands)
    }

    /// 丢弃全部排队命令且不提交。
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}
