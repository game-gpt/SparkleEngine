//! Spark GC：标记–清扫堆，供 `spark-vm` / `spark-script` 使用。
//!
//! 不变式：所有堆对象经 [`GcHandle`] 引用；根由宿主在 [`Heap::collect`] 前登记。

use std::collections::HashMap;

use thiserror::Error;

/// 堆对象句柄（分代可后续扩展；当前为槽位索引）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcHandle(pub u32);

/// 堆上可回收对象。
#[derive(Debug, Clone)]
pub enum GcObject {
    String(String),
    Array(Vec<Value>),
    Table(HashMap<String, Value>),
    /// 闭包：函数常量池下标 + 已捕获上值。
    Closure { func: u32, upvalues: Vec<Value> },
}

/// 运行时值（栈与槽共用）。
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    Handle(GcHandle),
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::Handle(_) => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Number(_) => "number",
            Self::Handle(_) => "object",
        }
    }
}

#[derive(Debug, Error)]
pub enum GcError {
    #[error("无效句柄 {0:?}")]
    BadHandle(GcHandle),
}

#[derive(Debug)]
struct Slot {
    obj: GcObject,
    marked: bool,
    live: bool,
}

/// 简单标记–清扫堆。
#[derive(Debug, Default)]
pub struct Heap {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// 分配计数，用于触发阈值。
    pub allocs_since_gc: usize,
    pub gc_threshold: usize,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            allocs_since_gc: 0,
            gc_threshold: 256,
        }
    }

    pub fn alloc(&mut self, obj: GcObject) -> GcHandle {
        self.allocs_since_gc += 1;
        if let Some(idx) = self.free.pop() {
            let slot = &mut self.slots[idx as usize];
            slot.obj = obj;
            slot.marked = false;
            slot.live = true;
            return GcHandle(idx);
        }
        let idx = self.slots.len() as u32;
        self.slots.push(Slot {
            obj,
            marked: false,
            live: true,
        });
        GcHandle(idx)
    }

    pub fn get(&self, h: GcHandle) -> Result<&GcObject, GcError> {
        self.slots
            .get(h.0 as usize)
            .filter(|s| s.live)
            .map(|s| &s.obj)
            .ok_or(GcError::BadHandle(h))
    }

    pub fn get_mut(&mut self, h: GcHandle) -> Result<&mut GcObject, GcError> {
        self.slots
            .get_mut(h.0 as usize)
            .filter(|s| s.live)
            .map(|s| &mut s.obj)
            .ok_or(GcError::BadHandle(h))
    }

    pub fn alloc_string(&mut self, s: impl Into<String>) -> Value {
        Value::Handle(self.alloc(GcObject::String(s.into())))
    }

    /// 从根集合标记并清扫。根由调用方提供（栈、全局、寄存器）。
    pub fn collect(&mut self, roots: &[Value]) {
        for slot in &mut self.slots {
            slot.marked = false;
        }
        let mut stack: Vec<GcHandle> = Vec::new();
        for v in roots {
            if let Value::Handle(h) = v {
                stack.push(*h);
            }
        }
        while let Some(h) = stack.pop() {
            let Some(slot) = self.slots.get_mut(h.0 as usize) else {
                continue;
            };
            if !slot.live || slot.marked {
                continue;
            }
            slot.marked = true;
            let children: Vec<GcHandle> = match &slot.obj {
                GcObject::String(_) => Vec::new(),
                GcObject::Array(items) => items
                    .iter()
                    .filter_map(|it| match it {
                        Value::Handle(ch) => Some(*ch),
                        _ => None,
                    })
                    .collect(),
                GcObject::Table(map) => map
                    .values()
                    .filter_map(|it| match it {
                        Value::Handle(ch) => Some(*ch),
                        _ => None,
                    })
                    .collect(),
                GcObject::Closure { upvalues, .. } => upvalues
                    .iter()
                    .filter_map(|it| match it {
                        Value::Handle(ch) => Some(*ch),
                        _ => None,
                    })
                    .collect(),
            };
            stack.extend(children);
        }
        self.free.clear();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if !slot.live {
                continue;
            }
            if slot.marked {
                slot.marked = false;
            } else {
                slot.live = false;
                // 释放内容
                slot.obj = GcObject::String(String::new());
                self.free.push(i as u32);
            }
        }
        self.allocs_since_gc = 0;
    }

    pub fn maybe_collect(&mut self, roots: &[Value]) {
        if self.allocs_since_gc >= self.gc_threshold {
            self.collect(roots);
        }
    }

    pub fn live_count(&self) -> usize {
        self.slots.iter().filter(|s| s.live).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_unreachable_string() {
        let mut heap = Heap::new();
        let keep = heap.alloc_string("keep");
        let _drop = heap.alloc_string("drop");
        assert_eq!(heap.live_count(), 2);
        heap.collect(&[keep]);
        assert_eq!(heap.live_count(), 1);
    }
}
