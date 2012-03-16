//! ECS 框架占位。后续承接 Archetype 存储与系统调度。

use spark_core::SparkError;

/// 稳定实体标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(pub u64);

/// 世界容器占位。
#[derive(Debug, Default)]
pub struct World {
    next_id: u64,
}

impl World {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    pub fn spawn_empty(&mut self) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        Entity(id)
    }

    pub fn entity_count_hint(&self) -> u64 {
        self.next_id.saturating_sub(1)
    }

    pub fn tick_placeholder(&mut self) -> Result<(), SparkError> {
        Ok(())
    }
}
