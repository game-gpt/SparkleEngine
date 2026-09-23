//! Rust 系统查询助手（薄封装 `spark-ecs` 列遍历）。

use spark_ecs::{Component, Entity};

use super::SystemContext;

impl<'a> SystemContext<'a> {
    /// 只读遍历所有含 `T` 的实体。
    pub fn query<T: Component>(&self, f: impl FnMut(Entity, &T)) {
        self.world.for_each::<T>(f);
    }

    /// 可变遍历所有含 `T` 的实体。
    pub fn query_mut<T: Component>(&mut self, f: impl FnMut(Entity, &mut T)) {
        self.world.for_each_mut::<T>(f);
    }

    /// 可变遍历同时含 `A` 与 `B` 的实体。
    pub fn query_mut2<A: Component, B: Component>(&mut self, f: impl FnMut(Entity, &mut A, &mut B)) {
        self.world.for_each2_mut::<A, B>(f);
    }
}
