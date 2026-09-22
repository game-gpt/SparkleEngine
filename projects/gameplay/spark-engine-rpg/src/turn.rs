//! 简易回合序（FIFO）。

use crate::party::ActorId;
use std::collections::VecDeque;

/// 回合队列：队首为当前行动者。
#[derive(Debug, Default)]
pub struct TurnOrder {
    queue: VecDeque<ActorId>,
}

impl TurnOrder {
    /// 将成员排到队尾。
    pub fn enqueue(&mut self, id: ActorId) {
        self.queue.push_back(id);
    }

    /// 当前行动者（队首）；空队列为 `None`。
    pub fn current(&self) -> Option<ActorId> {
        self.queue.front().copied()
    }

    /// 弹出队首并返回（结束其回合）；空队列为 `None`。
    pub fn advance(&mut self) -> Option<ActorId> {
        self.queue.pop_front()
    }

    /// 清空队列。
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// 队列长度。
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// 是否无人排队。
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
