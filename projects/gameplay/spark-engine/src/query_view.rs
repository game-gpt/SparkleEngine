//! 脚本查询视图：受控只读访问 ECS，禁止在遍历中改结构。
//!
//! 初版仅提供按 [`crate::ScriptArchetypeTag`] 的原型名过滤。
//! 组件名 → 稳定槽位解析接入后再扩展通用查询。
//!
//! VM **不**持有 [`World`]：宿主在同步点把世界拍成 [`ScriptQuerySnapshot`]，
//! 脚本经 `query_*` 原生只读快照。

use std::collections::HashMap;
use std::sync::Arc;

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

    /// 拍成可跨域共享的只读快照。
    pub fn to_snapshot(&self) -> ScriptQuerySnapshot {
        ScriptQuerySnapshot::from_view(self)
    }
}

/// 帧同步点拍摄的只读查询结果（脚本原生读取此表，不碰 `World`）。
#[derive(Debug, Clone, Default)]
pub struct ScriptQuerySnapshot {
    by_archetype: HashMap<Arc<str>, Vec<u64>>,
}

impl ScriptQuerySnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_world(world: &World) -> Self {
        ScriptQueryView::new(world).to_snapshot()
    }

    pub fn from_view(view: &ScriptQueryView<'_>) -> Self {
        let mut by_archetype: HashMap<Arc<str>, Vec<u64>> = HashMap::new();
        view.world.for_each::<ScriptArchetypeTag>(|e, tag| {
            by_archetype
                .entry(Arc::clone(&tag.name))
                .or_default()
                .push(e.to_bits());
        });
        Self { by_archetype }
    }

    pub fn clear(&mut self) {
        self.by_archetype.clear();
    }

    pub fn count(&self, archetype: &str) -> usize {
        self.by_archetype
            .get(archetype)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// 按下标取实体位模式；越界返回 `None`。
    pub fn entity_at(&self, archetype: &str, index: usize) -> Option<u64> {
        self.by_archetype
            .get(archetype)
            .and_then(|v| v.get(index).copied())
    }

    pub fn archetypes(&self) -> impl Iterator<Item = &str> {
        self.by_archetype.keys().map(|k| k.as_ref())
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

    #[test]
    fn snapshot_exposes_count_and_entity_at() {
        let mut world = World::new();
        apply_script_commands(
            &mut world,
            &[
                ScriptCommand::Spawn {
                    archetype: Arc::from("rock"),
                },
                ScriptCommand::Spawn {
                    archetype: Arc::from("rock"),
                },
            ],
        );
        let snap = ScriptQuerySnapshot::from_world(&world);
        assert_eq!(snap.count("rock"), 2);
        assert_eq!(snap.count("missing"), 0);
        assert!(snap.entity_at("rock", 0).is_some());
        assert!(snap.entity_at("rock", 2).is_none());
    }
}
