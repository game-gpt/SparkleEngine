//! 类型化事件总线。
//!
//! 每种事件类型各自一条双缓冲队列；帧边界调用 [`EventBus::update_all`]。
//! **不**定义游戏事件枚举。
//!
//! # 双缓冲不变式
//!
//! - **writing**：本帧发送端追加；读取端不应依赖其顺序直至 `update`。
//! - **reading**：上一帧 `update` 后的快照；[`Events::iter`] / [`Events::drain_reading`] 只看此缓冲。
//! - `update`：清空 reading，再与 writing 交换，使本帧写入变为下一帧可读。

#![forbid(missing_docs)]
use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

/// 单一事件类型的双缓冲队列。
///
/// 泛型 `E` 为事件载荷；同一类型在 [`EventBus`] 内只对应一条队列。
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
    /// 空队列：writing / reading 均为空。
    pub fn new() -> Self {
        Self { writing: Vec::new(), reading: Vec::new() }
    }

    /// 向 writing 追加一条事件（本帧不可经 [`Self::iter`] 读到）。
    pub fn send(&mut self, event: E) {
        self.writing.push(event);
    }

    /// 批量追加到 writing；迭代顺序即入队顺序。
    pub fn send_batch<I: IntoIterator<Item = E>>(&mut self, iter: I) {
        self.writing.extend(iter);
    }

    /// 当前 writing 缓冲中待交换的事件数。
    pub fn len_writing(&self) -> usize {
        self.writing.len()
    }

    /// 当前 reading 缓冲中可读的事件数（上一帧 `update` 结果）。
    pub fn len_reading(&self) -> usize {
        self.reading.len()
    }

    /// writing 与 reading 皆空时为真。
    pub fn is_empty(&self) -> bool {
        self.writing.is_empty() && self.reading.is_empty()
    }

    /// 按入队顺序迭代 **reading**（不含本帧 writing）。
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.reading.iter()
    }

    /// writing → reading，writing 清空。
    pub fn update(&mut self) {
        self.reading.clear();
        std::mem::swap(&mut self.writing, &mut self.reading);
    }

    /// 取走全部 reading 并置空；返回值顺序与原先 `iter` 一致。
    pub fn drain_reading(&mut self) -> Vec<E> {
        std::mem::take(&mut self.reading)
    }

    /// 取走全部 writing 并置空（丢弃本帧未交换事件时使用）。
    pub fn drain_writing(&mut self) -> Vec<E> {
        std::mem::take(&mut self.writing)
    }

    /// 清空 writing 与 reading；不保留任何待处理事件。
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
///
/// 按 `TypeId` 惰性创建 [`Events<E>`]；跨类型之间互不影响。
/// 帧末应调用 [`Self::update_all`]，使本帧发送对下一帧可读。
#[derive(Default)]
pub struct EventBus {
    queues: HashMap<TypeId, Box<dyn ErasedQueue>>,
}

impl EventBus {
    /// 空总线，尚无任何事件类型队列。
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure<E: Send + Sync + 'static>(&mut self) -> &mut Events<E> {
        let id = TypeId::of::<E>();
        self.queues.entry(id).or_insert_with(|| Box::new(Events::<E>::new()));
        self.queues.get_mut(&id).unwrap().as_any_mut().downcast_mut::<Events<E>>().expect("事件队列类型")
    }

    /// 向类型 `E` 的 writing 发送一条；若队列不存在则创建。
    pub fn send<E: Send + Sync + 'static>(&mut self, event: E) {
        self.ensure::<E>().send(event);
    }

    /// 只读访问类型 `E` 的队列；从未发送过该类型时返回 `None`。
    pub fn events<E: Send + Sync + 'static>(&self) -> Option<&Events<E>> {
        self.queues.get(&TypeId::of::<E>()).and_then(|q| q.as_any().downcast_ref::<Events<E>>())
    }

    /// 可变访问类型 `E` 的队列；不存在时创建空队列。
    pub fn events_mut<E: Send + Sync + 'static>(&mut self) -> &mut Events<E> {
        self.ensure::<E>()
    }

    /// 所有类型：writing → reading。
    pub fn update_all(&mut self) {
        for q in self.queues.values_mut() {
            q.update();
        }
    }

    /// 清空所有类型队列的 writing 与 reading；保留已注册的类型槽位。
    pub fn clear_all(&mut self) {
        for q in self.queues.values_mut() {
            q.clear();
        }
    }

    /// 已注册事件类型数（`TypeId` 条目数）。
    pub fn type_count(&self) -> usize {
        self.queues.len()
    }

    /// 所有类型 writing + reading 事件总数。
    pub fn len(&self) -> usize {
        self.queues.values().map(|q| q.total_len()).sum()
    }

    /// 等同 [`Self::clear_all`]。
    pub fn clear(&mut self) {
        self.clear_all();
    }
}
