//! Rust 域结构变更命令：系统执行期排队，相位边界一次性提交。

use spark_ecs::{Component, Entity, World};

/// 单相位内共享的结构变更队列（由 [`super::RuntimeScheduler`] 在相位末 [`Self::apply`]）。
#[derive(Default)]
pub struct RustCommands {
    pending: Vec<Box<dyn FnOnce(&mut World) + Send>>,
}

impl RustCommands {
    /// 空队列。
    pub fn new() -> Self {
        Self::default()
    }

    /// 是否尚无排队命令。
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// 排队销毁实体。
    pub fn despawn(&mut self, entity: Entity) {
        self.pending.push(Box::new(move |world| {
            world.despawn(entity);
        }));
    }

    /// 排队生成仅含单个组件 `T` 的实体。
    pub fn spawn<T: Component>(&mut self, value: T) {
        self.pending.push(Box::new(move |world| {
            world.spawn(value);
        }));
    }

    /// 排队插入或覆盖组件。
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.pending.push(Box::new(move |world| {
            world.insert(entity, value);
        }));
    }

    /// 按注册顺序提交全部排队命令。
    pub fn apply(&mut self, world: &mut World) {
        let pending = std::mem::take(&mut self.pending);
        for cmd in pending {
            cmd(world);
        }
    }
}
