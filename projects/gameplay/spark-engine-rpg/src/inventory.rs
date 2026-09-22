//! 堆叠背包（物品 ID 为不透明字符串）。

use spark_types::{ErrorArg, ErrorCode, SparkError};

fn inventory_full() -> ErrorCode {
    ErrorCode::new("spark.rpg", "inventory.full")
}

fn inventory_invalid() -> ErrorCode {
    ErrorCode::new("spark.rpg", "inventory.invalid")
}

/// 单格堆叠：同 `id` 可合并计数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStack {
    /// 游戏仓定义的物品键（非显示名）。
    pub id: String,
    /// 堆叠数量（`add` / `remove` 维护；为 0 的格会被清空为 `None`）。
    pub count: u32,
}

/// 固定槽位数的堆叠背包。
///
/// 先尝试合并到已有同 `id` 堆，再占用空槽；无空槽时返回 `spark.rpg.inventory.full`。
#[derive(Debug, Clone)]
pub struct Inventory {
    slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    /// 创建 `n` 个空槽（`n` 至少为 1）。
    pub fn with_slots(n: usize) -> Self {
        Self { slots: vec![None; n.max(1)] }
    }

    /// 只读槽位切片（`None` 为空槽）。
    pub fn slots(&self) -> &[Option<ItemStack>] {
        &self.slots
    }

    /// 统计某 `id` 在所有槽中的总数量。
    pub fn count(&self, id: &str) -> u32 {
        self.slots.iter().filter_map(|s| s.as_ref()).filter(|s| s.id == id).map(|s| s.count).sum()
    }

    /// 加入 `count` 个物品；`count == 0` 为成功空操作。
    ///
    /// 失败：背包满 → `spark.rpg.inventory.full`。
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
        Err(SparkError::new(inventory_full()))
    }

    /// 扣除 `count` 个物品；不足时返回 `spark.rpg.inventory.invalid`（带 `need`/`have`）。
    pub fn remove(&mut self, id: &str, count: u32) -> Result<(), SparkError> {
        let have = self.count(id);
        if have < count {
            return Err(SparkError::new(inventory_invalid())
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
                }
                else {
                    stack.count -= left;
                    left = 0;
                }
            }
        }
        Ok(())
    }
}
