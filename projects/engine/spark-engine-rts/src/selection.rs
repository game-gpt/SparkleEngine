//! 框选集合。

use crate::unit::UnitId;

#[derive(Debug, Default, Clone)]
pub struct Selection {
    ids: Vec<UnitId>,
}

impl Selection {
    pub fn clear(&mut self) {
        self.ids.clear();
    }

    pub fn set(&mut self, ids: Vec<UnitId>) {
        self.ids = ids;
    }

    pub fn add(&mut self, id: UnitId) {
        if !self.ids.contains(&id) {
            self.ids.push(id);
        }
    }

    pub fn remove(&mut self, id: UnitId) {
        self.ids.retain(|x| *x != id);
    }

    pub fn ids(&self) -> &[UnitId] {
        &self.ids
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}
