//! 脚本领域事件收件箱：ECS / 宿主事件先入队，再在允许的 phase 批量派发。
//!
//! 禁止以任意同步回调嵌套重入 VM。派发顺序在同一 tick 内保持稳定。

use std::sync::Arc;

use spark_gc::Value;

/// 入队事件（初版：名字 + 载荷值列表）。
#[derive(Debug, Clone)]
pub struct ScriptEvent {
    pub name: Arc<str>,
    pub args: Vec<Value>,
}

/// 每领域一份事件 inbox。
#[derive(Debug, Default, Clone)]
pub struct ScriptEventInbox {
    events: Vec<ScriptEvent>,
}

impl ScriptEventInbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn push(&mut self, name: impl Into<Arc<str>>, args: Vec<Value>) {
        self.events.push(ScriptEvent { name: name.into(), args });
    }

    /// 取出全部事件并清空（供 phase 批量派发）。
    pub fn drain(&mut self) -> Vec<ScriptEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_preserves_order() {
        let mut inbox = ScriptEventInbox::new();
        inbox.push("a", vec![Value::Number(1.0)]);
        inbox.push("b", vec![Value::Number(2.0)]);
        let ev = inbox.drain();
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].name.as_ref(), "a");
        assert_eq!(ev[1].name.as_ref(), "b");
        assert!(inbox.is_empty());
    }
}
