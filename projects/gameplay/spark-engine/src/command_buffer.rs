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
        archetype: Arc<str>,
    },
    /// 销毁实体（代际句柄，提交期校验）。
    Despawn {
        entity: u64,
    },
    /// 添加组件（组件槽在链接期解析；此处暂用名字）。
    AddComponent {
        entity: u64,
        component: Arc<str>,
    },
    /// 移除组件。
    RemoveComponent {
        entity: u64,
        component: Arc<str>,
    },
}

/// 每领域一份命令缓冲。同步点 [`drain`] 后交给引擎提交。
#[derive(Debug, Default, Clone)]
pub struct ScriptCommandBuffer {
    commands: Vec<ScriptCommand>,
}

impl ScriptCommandBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn push(&mut self, cmd: ScriptCommand) {
        self.commands.push(cmd);
    }

    pub fn spawn(&mut self, archetype: impl Into<Arc<str>>) {
        self.push(ScriptCommand::Spawn {
            archetype: archetype.into(),
        });
    }

    pub fn despawn(&mut self, entity: u64) {
        self.push(ScriptCommand::Despawn { entity });
    }

    pub fn add_component(&mut self, entity: u64, component: impl Into<Arc<str>>) {
        self.push(ScriptCommand::AddComponent {
            entity,
            component: component.into(),
        });
    }

    pub fn remove_component(&mut self, entity: u64, component: impl Into<Arc<str>>) {
        self.push(ScriptCommand::RemoveComponent {
            entity,
            component: component.into(),
        });
    }

    /// 取出全部命令并清空缓冲（供同步点提交）。
    pub fn drain(&mut self) -> Vec<ScriptCommand> {
        std::mem::take(&mut self.commands)
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_clears_buffer() {
        let mut buf = ScriptCommandBuffer::new();
        buf.spawn("unit");
        buf.despawn(1);
        assert_eq!(buf.len(), 2);
        let cmds = buf.drain();
        assert_eq!(cmds.len(), 2);
        assert!(buf.is_empty());
        assert!(matches!(cmds[0], ScriptCommand::Spawn { .. }));
    }
}
