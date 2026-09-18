//! 堆叠背包（物品 ID 为不透明字符串）。

use spark_core::{ErrorArg, SparkError, codes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStack {
    pub id: String,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct Inventory {
    slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn with_slots(n: usize) -> Self {
        Self {
            slots: vec![None; n.max(1)],
        }
    }

    pub fn slots(&self) -> &[Option<ItemStack>] {
        &self.slots
    }

    pub fn count(&self, id: &str) -> u32 {
        self.slots
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|s| s.id == id)
            .map(|s| s.count)
            .sum()
    }

    pub fn add(&mut self, id: impl Into<String>, count: u32) -> Result<(), SparkError> {
        if count == 0 {
            return Ok(());
        }
        let id = id.into();
        for slot in &mut self.slots {
            if let Some(stack) = slot {
                if stack.id == id {
                    stack.count = stack.count.saturating_add(count);
                    return Ok(());
                }
            }
        }
        for slot in &mut self.slots {
            if slot.is_none() {
                *slot = Some(ItemStack { id, count });
                return Ok(());
            }
        }
        Err(SparkError::new(codes::inventory_full()))
    }

    pub fn remove(&mut self, id: &str, count: u32) -> Result<(), SparkError> {
        let have = self.count(id);
        if have < count {
            return Err(SparkError::new(codes::inventory_invalid())
                .arg("reason", ErrorArg::String("insufficient".into()))
                .arg("need", ErrorArg::Unsigned(count as u64))
                .arg("have", ErrorArg::Unsigned(have as u64))
                .arg("item", ErrorArg::String(id.into())));
        }
        let mut left = count;
        for slot in &mut self.slots {
            if left == 0 {
                break;
            }
            if let Some(stack) = slot {
                if stack.id != id {
                    continue;
                }
                if stack.count <= left {
                    left -= stack.count;
                    *slot = None;
                } else {
                    stack.count -= left;
                    left = 0;
                }
            }
        }
        Ok(())
    }
}
