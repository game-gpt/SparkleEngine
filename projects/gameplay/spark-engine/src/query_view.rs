//! 脚本查询视图：受控只读访问 ECS，禁止在遍历中改结构。
//!
//! 初版仅提供按 [`crate::ScriptArchetypeTag`] 的原型名过滤。
//! 组件名 → 稳定槽位解析接入后再扩展通用查询。

use spark_ecs::{Entity, World};

use crate::command_apply::ScriptArchetypeTag;

/// 只读查询视图（不持有写权限，结构变更须走命令缓冲）。
pub struct ScriptQueryView<'a> {
    world: &'a World,
}

impl<'a> ScriptQueryView<'a> {
    pub fn new(world: &'a World) -> Self {
        Self { world }
    }

    pub fn world(&self) -> &'a World {
        self.world
    }

    /// 收集带脚本原型标记且名字匹配的实体。
    pub fn entities_with_archetype(&self, archetype: &str) -> Vec<Entity> {
        let mut out = Vec::new();
        self.world.for_each::<ScriptArchetypeTag>(|e, tag| {
            if tag.name.as_ref() == archetype {
                out.push(e);
            }
        });
        out
    }

    /// 读取实体的脚本原型名（若有）。
    pub fn archetype_of(&self, entity: Entity) -> Option<&str> {
        self.world
            .get::<ScriptArchetypeTag>(entity)
            .map(|t| t.name.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_apply::apply_script_commands;
    use crate::command_buffer::ScriptCommand;
    use std::sync::Arc;

    #[test]
    fn query_filters_by_archetype_name() {
        let mut world = World::new();
        apply_script_commands(
            &mut world,
            &[
                ScriptCommand::Spawn {
                    archetype: Arc::from("rock"),
                },
                ScriptCommand::Spawn {
                    archetype: Arc::from("tree"),
                },
                ScriptCommand::Spawn {
                    archetype: Arc::from("rock"),
                },
            ],
        );
        let view = ScriptQueryView::new(&world);
        assert_eq!(view.entities_with_archetype("rock").len(), 2);
        assert_eq!(view.entities_with_archetype("tree").len(), 1);
        assert!(view.entities_with_archetype("missing").is_empty());
    }
}
