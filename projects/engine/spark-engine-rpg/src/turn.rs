//! 简易回合序（FIFO）。

use crate::party::ActorId;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct TurnOrder {
    queue: VecDeque<ActorId>,
}

impl TurnOrder {
    pub fn enqueue(&mut self, id: ActorId) {
        self.queue.push_back(id);
    }

    pub fn current(&self) -> Option<ActorId> {
        self.queue.front().copied()
    }

    pub fn advance(&mut self) -> Option<ActorId> {
        self.queue.pop_front()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
