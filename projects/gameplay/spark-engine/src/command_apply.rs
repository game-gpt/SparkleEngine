//! 将脚本命令缓冲提交到 ECS [`World`]。
//!
//! 初版：`Spawn` 生成带 [`ScriptArchetypeTag`] 的实体；`Despawn` 按位模式句柄销毁。
//! 按名增删组件尚需组件描述符槽位，暂记入 [`CommandApplyReport::skipped_named_components`]。

use std::sync::Arc;

use spark_ecs::{Entity, World};

use crate::command_buffer::ScriptCommand;

/// 脚本原型标记组件：提交期占位，游戏层可展开为真实原型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptArchetypeTag {
    pub name: Arc<str>,
}

/// 单次提交报告。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommandApplyReport {
    pub spawned: Vec<Entity>,
    pub despawned: Vec<Entity>,
    pub skipped_named_components: usize,
    pub failed_despawns: usize,
}

impl CommandApplyReport {
    pub fn merge(&mut self, other: CommandApplyReport) {
        self.spawned.extend(other.spawned);
        self.despawned.extend(other.despawned);
        self.skipped_named_components += other.skipped_named_components;
        self.failed_despawns += other.failed_despawns;
    }
}

/// 将一批脚本命令应用到世界。
pub fn apply_script_commands(
    world: &mut World,
    commands: &[ScriptCommand],
) -> CommandApplyReport {
    let mut report = CommandApplyReport::default();
    for cmd in commands {
        match cmd {
            ScriptCommand::Spawn { archetype } => {
                let e = world.spawn(ScriptArchetypeTag {
                    name: Arc::clone(archetype),
                });
                report.spawned.push(e);
            }
            ScriptCommand::Despawn { entity } => {
                let e = Entity::from_bits(*entity);
                if world.despawn(e) {
                    report.despawned.push(e);
                } else {
                    report.failed_despawns += 1;
                }
            }
            ScriptCommand::AddComponent { .. } | ScriptCommand::RemoveComponent { .. } => {
                report.skipped_named_components += 1;
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn spawn_and_despawn_apply() {
        let mut world = World::new();
        let report = apply_script_commands(
            &mut world,
            &[ScriptCommand::Spawn {
                archetype: Arc::from("rock"),
            }],
        );
        assert_eq!(report.spawned.len(), 1);
        let e = report.spawned[0];
        assert_eq!(
            world.get::<ScriptArchetypeTag>(e).map(|t| t.name.as_ref()),
            Some("rock")
        );
        let report2 = apply_script_commands(
            &mut world,
            &[ScriptCommand::Despawn {
                entity: e.to_bits(),
            }],
        );
        assert_eq!(report2.despawned, vec![e]);
        assert!(!world.is_alive(e));
    }

    #[test]
    fn named_component_edits_are_skipped() {
        let mut world = World::new();
        let report = apply_script_commands(
            &mut world,
            &[ScriptCommand::AddComponent {
                entity: 0,
                component: Arc::from("Health"),
            }],
        );
        assert_eq!(report.skipped_named_components, 1);
    }
}
