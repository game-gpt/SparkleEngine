//! 类型化事件总线。
//!
//! 每种事件类型各自一条双缓冲队列；帧边界调用 [`EventBus::update_all`]。
//! **不**定义游戏事件枚举。

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

/// 单一事件类型的双缓冲队列。
#[derive(Debug)]
pub struct Events<E> {
    writing: Vec<E>,
    reading: Vec<E>,
}

impl<E> Default for Events<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Events<E> {
    pub fn new() -> Self {
        Self { writing: Vec::new(), reading: Vec::new() }
    }

    pub fn send(&mut self, event: E) {
        self.writing.push(event);
    }

    pub fn send_batch<I: IntoIterator<Item = E>>(&mut self, iter: I) {
        self.writing.extend(iter);
    }

    pub fn len_writing(&self) -> usize {
        self.writing.len()
    }

    pub fn len_reading(&self) -> usize {
        self.reading.len()
    }

    pub fn is_empty(&self) -> bool {
        self.writing.is_empty() && self.reading.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.reading.iter()
    }

    /// writing → reading，writing 清空。
    pub fn update(&mut self) {
        self.reading.clear();
        std::mem::swap(&mut self.writing, &mut self.reading);
    }

    pub fn drain_reading(&mut self) -> Vec<E> {
        std::mem::take(&mut self.reading)
    }

    pub fn drain_writing(&mut self) -> Vec<E> {
        std::mem::take(&mut self.writing)
    }

    pub fn clear(&mut self) {
        self.writing.clear();
        self.reading.clear();
    }
}

trait ErasedQueue: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn update(&mut self);
    fn clear(&mut self);
    fn total_len(&self) -> usize;
}

impl<E: Send + Sync + 'static> ErasedQueue for Events<E> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn update(&mut self) {
        Events::update(self);
    }

    fn clear(&mut self) {
        Events::clear(self);
    }

    fn total_len(&self) -> usize {
        self.writing.len() + self.reading.len()
    }
}

/// 多类型事件总线。
#[derive(Default)]
pub struct EventBus {
    queues: HashMap<TypeId, Box<dyn ErasedQueue>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure<E: Send + Sync + 'static>(&mut self) -> &mut Events<E> {
        let id = TypeId::of::<E>();
        self.queues.entry(id).or_insert_with(|| Box::new(Events::<E>::new()));
        self.queues.get_mut(&id).unwrap().as_any_mut().downcast_mut::<Events<E>>().expect("事件队列类型")
    }

    pub fn send<E: Send + Sync + 'static>(&mut self, event: E) {
        self.ensure::<E>().send(event);
    }

    pub fn events<E: Send + Sync + 'static>(&self) -> Option<&Events<E>> {
        self.queues.get(&TypeId::of::<E>()).and_then(|q| q.as_any().downcast_ref::<Events<E>>())
    }

    pub fn events_mut<E: Send + Sync + 'static>(&mut self) -> &mut Events<E> {
        self.ensure::<E>()
    }

    /// 所有类型：writing → reading。
    pub fn update_all(&mut self) {
        for q in self.queues.values_mut() {
            q.update();
        }
    }

    pub fn clear_all(&mut self) {
        for q in self.queues.values_mut() {
            q.clear();
        }
    }

    pub fn type_count(&self) -> usize {
        self.queues.len()
    }

    pub fn len(&self) -> usize {
        self.queues.values().map(|q| q.total_len()).sum()
    }

    pub fn clear(&mut self) {
        self.clear_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Boom(u32);

    #[test]
    fn typed_double_buffer() {
        let mut bus = EventBus::new();
        bus.send(Boom(1));
        bus.send(Boom(2));
        assert_eq!(bus.events::<Boom>().unwrap().len_writing(), 2);
        assert_eq!(bus.events::<Boom>().unwrap().len_reading(), 0);

        bus.update_all();
        let got: Vec<_> = bus.events::<Boom>().unwrap().iter().map(|b| b.0).collect();
        assert_eq!(got, vec![1, 2]);
        assert_eq!(bus.events::<Boom>().unwrap().len_writing(), 0);

        bus.send(Boom(3));
        bus.update_all();
        let got: Vec<_> = bus.events::<Boom>().unwrap().iter().map(|b| b.0).collect();
        assert_eq!(got, vec![3]);
    }

    #[test]
    fn multiple_types() {
        let mut bus = EventBus::new();
        bus.send(Boom(9));
        bus.send("hi".to_string());
        assert_eq!(bus.type_count(), 2);
        bus.update_all();
        assert_eq!(bus.events::<String>().unwrap().iter().next().unwrap(), "hi");
    }
}
