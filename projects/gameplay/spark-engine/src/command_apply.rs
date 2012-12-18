//! 将脚本命令缓冲提交到 ECS [`World`]。
//!
//! `Spawn` 生成带 [`ScriptArchetypeTag`] 的实体；`Despawn` 按位模式句柄销毁。
//! 按名增删组件经 [`ScriptComponentCatalog`] 解析为稳定 [`ComponentDescriptorId`]。
//! 未知或尚无类型体的组件名必须报错，不得静默跳过。

use std::{collections::HashMap, sync::Arc};

use spark_ecs::{Entity, World};

use crate::command_buffer::ScriptCommand;

/// 脚本侧组件描述符槽位（与宿主登记表顺序一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentDescriptorId(pub u32);

/// 脚本原型标记组件：提交期占位，游戏层可展开为真实原型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptArchetypeTag {
    pub name: Arc<str>,
}

/// 脚本可挂载的空标记组件（验证按名增删路径）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptMarker;

/// 内建可应用组件名。
pub const SCRIPT_MARKER_NAME: &str = "ScriptMarker";

/// 脚本组件名 → 描述符槽。
#[derive(Debug, Clone, Default)]
pub struct ScriptComponentCatalog {
    by_name: HashMap<Arc<str>, ComponentDescriptorId>,
    names: Vec<Arc<str>>,
}

impl ScriptComponentCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// 引擎默认：登记 [`SCRIPT_MARKER_NAME`]。
    pub fn with_builtins() -> Self {
        let mut catalog = Self::new();
        catalog.register(SCRIPT_MARKER_NAME);
        catalog
    }

    pub fn register(&mut self, name: impl Into<Arc<str>>) -> ComponentDescriptorId {
        let name = name.into();
        if let Some(&id) = self.by_name.get(&name) {
            return id;
        }
        let id = ComponentDescriptorId(self.names.len() as u32);
        self.by_name.insert(Arc::clone(&name), id);
        self.names.push(name);
        id
    }

    pub fn id_of(&self, name: &str) -> Option<ComponentDescriptorId> {
        self.by_name.get(name).copied()
    }

    pub fn name_of(&self, id: ComponentDescriptorId) -> Option<&str> {
        self.names.get(id.0 as usize).map(|s| s.as_ref())
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// 组件应用失败（稳定机器令牌）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandApplyError {
    UnknownComponent { component: Arc<str> },
    UnsupportedComponent { component: Arc<str> },
    EntityNotAlive { entity: u64, component: Arc<str> },
}

impl CommandApplyError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownComponent { .. } => "spark.engine.command.unknown_component",
            Self::UnsupportedComponent { .. } => "spark.engine.command.unsupported_component",
            Self::EntityNotAlive { .. } => "spark.engine.command.entity_not_alive",
        }
    }
}

impl std::fmt::Display for CommandApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for CommandApplyError {}

/// 单次提交报告。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommandApplyReport {
    pub spawned: Vec<Entity>,
    pub despawned: Vec<Entity>,
    pub added_components: usize,
    pub removed_components: usize,
    pub failed_despawns: usize,
}

impl CommandApplyReport {
    pub fn merge(&mut self, other: CommandApplyReport) {
        self.spawned.extend(other.spawned);
        self.despawned.extend(other.despawned);
        self.added_components += other.added_components;
        self.removed_components += other.removed_components;
        self.failed_despawns += other.failed_despawns;
    }
}

/// 将一批脚本命令应用到世界（使用内建组件目录）。
pub fn apply_script_commands(world: &mut World, commands: &[ScriptCommand]) -> Result<CommandApplyReport, CommandApplyError> {
    apply_script_commands_with(world, commands, &ScriptComponentCatalog::with_builtins())
}

/// 将一批脚本命令应用到世界（显式组件目录）。
pub fn apply_script_commands_with(
    world: &mut World,
    commands: &[ScriptCommand],
    catalog: &ScriptComponentCatalog,
) -> Result<CommandApplyReport, CommandApplyError> {
    let mut report = CommandApplyReport::default();
    for cmd in commands {
        match cmd {
            ScriptCommand::Spawn { archetype } => {
                let e = world.spawn(ScriptArchetypeTag { name: Arc::clone(archetype) });
                report.spawned.push(e);
            }
            ScriptCommand::Despawn { entity } => {
                let e = Entity::from_bits(*entity);
                if world.despawn(e) {
                    report.despawned.push(e);
                }
                else {
                    report.failed_despawns += 1;
                }
            }
            ScriptCommand::AddComponent { entity, component } => {
                let e = Entity::from_bits(*entity);
                match catalog.id_of(component) {
                    Some(id) if catalog.name_of(id) == Some(SCRIPT_MARKER_NAME) => {
                        if world.insert(e, ScriptMarker) {
                            report.added_components += 1;
                        }
                        else {
                            return Err(CommandApplyError::EntityNotAlive { entity: *entity, component: Arc::clone(component) });
                        }
                    }
                    Some(_) => {
                        return Err(CommandApplyError::UnsupportedComponent { component: Arc::clone(component) });
                    }
                    None => {
                        return Err(CommandApplyError::UnknownComponent { component: Arc::clone(component) });
                    }
                }
            }
            ScriptCommand::RemoveComponent { entity, component } => {
                let e = Entity::from_bits(*entity);
                match catalog.id_of(component) {
                    Some(id) if catalog.name_of(id) == Some(SCRIPT_MARKER_NAME) => {
                        if world.remove::<ScriptMarker>(e).is_some() {
                            report.removed_components += 1;
                        }
                        else {
                            return Err(CommandApplyError::EntityNotAlive { entity: *entity, component: Arc::clone(component) });
                        }
                    }
                    Some(_) => {
                        return Err(CommandApplyError::UnsupportedComponent { component: Arc::clone(component) });
                    }
                    None => {
                        return Err(CommandApplyError::UnknownComponent { component: Arc::clone(component) });
                    }
                }
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn spawn_and_despawn_apply() {
        let mut world = World::new();
        let report = apply_script_commands(&mut world, &[ScriptCommand::Spawn { archetype: Arc::from("rock") }]).unwrap();
        assert_eq!(report.spawned.len(), 1);
        let e = report.spawned[0];
        assert_eq!(world.get::<ScriptArchetypeTag>(e).map(|t| t.name.as_ref()), Some("rock"));
        let report2 = apply_script_commands(&mut world, &[ScriptCommand::Despawn { entity: e.to_bits() }]).unwrap();
        assert_eq!(report2.despawned, vec![e]);
        assert!(!world.is_alive(e));
    }

    #[test]
    fn unknown_component_edits_fail() {
        let mut world = World::new();
        let err = apply_script_commands(&mut world, &[ScriptCommand::AddComponent { entity: 0, component: Arc::from("Health") }]).unwrap_err();
        assert!(matches!(
            err,
            CommandApplyError::UnknownComponent { ref component } if component.as_ref() == "Health"
        ));
    }

    #[test]
    fn script_marker_add_and_remove() {
        let mut world = World::new();
        let spawn = apply_script_commands(&mut world, &[ScriptCommand::Spawn { archetype: Arc::from("unit") }]).unwrap();
        let e = spawn.spawned[0];
        let catalog = ScriptComponentCatalog::with_builtins();
        let added = apply_script_commands_with(
            &mut world,
            &[ScriptCommand::AddComponent { entity: e.to_bits(), component: Arc::from(SCRIPT_MARKER_NAME) }],
            &catalog,
        )
        .unwrap();
        assert_eq!(added.added_components, 1);
        assert!(world.get::<ScriptMarker>(e).is_some());
        let removed = apply_script_commands_with(
            &mut world,
            &[ScriptCommand::RemoveComponent { entity: e.to_bits(), component: Arc::from(SCRIPT_MARKER_NAME) }],
            &catalog,
        )
        .unwrap();
        assert_eq!(removed.removed_components, 1);
        assert!(world.get::<ScriptMarker>(e).is_none());
    }
}
