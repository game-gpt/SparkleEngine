//! 脚本领域事件收件箱：ECS / 宿主事件先入队，再在允许的 phase 批量派发。
//!
//! 禁止以任意同步回调嵌套重入 VM。派发顺序在同一 tick 内保持稳定。

use std::sync::Arc;

use spark_gc::Value;

/// 入队事件（初版：名字 + 载荷值列表）。
#[derive(Debug, Clone)]
pub struct ScriptEvent {
    /// 事件名；派发时优先走 `on_event`，否则尝试同名导出。
    pub name: Arc<str>,
    /// 传给脚本导出的参数列表（GC 值，跨调用需注意生命周期）。
    pub args: Vec<Value>,
}

/// 每领域一份事件 inbox。
#[derive(Debug, Default, Clone)]
pub struct ScriptEventInbox {
    events: Vec<ScriptEvent>,
}

impl ScriptEventInbox {
    /// 空收件箱。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前排队事件数。
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// 是否无待派发事件。
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// 追加一条事件（不立即进入 VM）。
    pub fn push(&mut self, name: impl Into<Arc<str>>, args: Vec<Value>) {
        self.events.push(ScriptEvent { name: name.into(), args });
    }

    /// 取出全部事件并清空（供 phase 批量派发）。
    pub fn drain(&mut self) -> Vec<ScriptEvent> {
        std::mem::take(&mut self.events)
    }

    /// 丢弃全部排队事件且不派发。
    pub fn clear(&mut self) {
        self.events.clear();
    }
}
