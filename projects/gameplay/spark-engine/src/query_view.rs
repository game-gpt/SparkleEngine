//! 脚本查询视图：受控只读访问 ECS，禁止在遍历中改结构。
//!
//! 世界在同步点拍成全量 [`ScriptQuerySnapshot`]（`query_base`）。
//! 每次脚本 System 调用再按描述符安装受限视图到 `query`，供 `query_*` 读取。
//!
//! VM **不**持有 [`World`]。

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use spark_ecs::{Entity, World};

use crate::command_apply::ScriptArchetypeTag;

/// 只读查询视图（不持有写权限，结构变更须走命令缓冲）。
pub struct ScriptQueryView<'a> {
    world: &'a World,
}

impl<'a> ScriptQueryView<'a> {
    /// 借用世界构造视图；调用方保证不以本视图为借口做结构变更。
    pub fn new(world: &'a World) -> Self {
        Self { world }
    }

    /// 底层世界只读引用。
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
        self.world.get::<ScriptArchetypeTag>(entity).map(|t| t.name.as_ref())
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
    /// 空快照。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从世界立即拍摄（遍历全部 [`ScriptArchetypeTag`]）。
    pub fn from_world(world: &World) -> Self {
        ScriptQueryView::new(world).to_snapshot()
    }

    /// 从已有视图拍摄。
    pub fn from_view(view: &ScriptQueryView<'_>) -> Self {
        let mut by_archetype: HashMap<Arc<str>, Vec<u64>> = HashMap::new();
        view.world.for_each::<ScriptArchetypeTag>(|e, tag| {
            by_archetype.entry(Arc::clone(&tag.name)).or_default().push(e.to_bits());
        });
        Self { by_archetype }
    }

    /// 清空全部原型桶。
    pub fn clear(&mut self) {
        self.by_archetype.clear();
    }

    /// 某原型下实体数量；未知原型为 0。
    pub fn count(&self, archetype: &str) -> usize {
        self.by_archetype.get(archetype).map(|v| v.len()).unwrap_or(0)
    }

    /// 按下标取实体位模式；越界返回 `None`。
    pub fn entity_at(&self, archetype: &str, index: usize) -> Option<u64> {
        self.by_archetype.get(archetype).and_then(|v| v.get(index).copied())
    }

    /// 快照中出现的全部原型名。
    pub fn archetypes(&self) -> impl Iterator<Item = &str> {
        self.by_archetype.keys().map(|k| k.as_ref())
    }

    /// 仅保留 `allow` 中的原型；`allow` 为空则原样克隆。
    pub fn filtered_by_archetypes(&self, allow: &HashSet<Arc<str>>) -> Self {
        if allow.is_empty() {
            return self.clone();
        }
        let mut by_archetype = HashMap::new();
        for (name, entities) in &self.by_archetype {
            if allow.iter().any(|a| a.as_ref() == name.as_ref()) {
                by_archetype.insert(Arc::clone(name), entities.clone());
            }
        }
        Self { by_archetype }
    }

    /// 原型是否在本快照中可见。
    pub fn contains_archetype(&self, archetype: &str) -> bool {
        self.by_archetype.contains_key(archetype)
    }
}
