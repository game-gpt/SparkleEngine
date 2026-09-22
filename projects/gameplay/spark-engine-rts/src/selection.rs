//! 框选集合。

use crate::unit::UnitId;

/// 当前框选 / 点选的单位集合（有序，去重）。
#[derive(Debug, Default, Clone)]
pub struct Selection {
    ids: Vec<UnitId>,
}

impl Selection {
    /// 清空选中。
    pub fn clear(&mut self) {
        self.ids.clear();
    }

    /// 用新列表整体替换选中（调用方负责去重亦可；本函数不额外去重）。
    pub fn set(&mut self, ids: Vec<UnitId>) {
        self.ids = ids;
    }

    /// 追加一个单位（已存在则忽略）。
    pub fn add(&mut self, id: UnitId) {
        if !self.ids.contains(&id) {
            self.ids.push(id);
        }
    }

    /// 从选中移除。
    pub fn remove(&mut self, id: UnitId) {
        self.ids.retain(|x| *x != id);
    }

    /// 当前选中句柄切片。
    pub fn ids(&self) -> &[UnitId] {
        &self.ids
    }

    /// 是否无选中。
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}
