//! 类型擦除事件队列占位。

#![forbid(unsafe_code)]

use std::any::Any;

#[derive(Default)]
pub struct EventBus {
    queue: Vec<Box<dyn Any + Send + Sync>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push<T: Any + Send + Sync>(&mut self, event: T) {
        self.queue.push(Box::new(event));
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}
