//! Spark GC：标记–清扫堆，供 `spark-vm` / `spark-script` 使用。
//!
//! 不变式：所有堆对象经 [`GcHandle`] 引用；根由宿主在 [`Heap::collect`] 前登记。
//! 栈值 [`Value`] 含非堆变体（数字、实体 ID、函数下标），GC 只追踪 [`Value::Handle`]。

use std::collections::HashMap;
use std::fmt;

/// 堆对象句柄（分代可后续扩展；当前为槽位索引）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcHandle(pub u32);

/// 堆上可回收对象。
#[derive(Debug, Clone)]
pub enum GcObject {
    String(String),
    Array(Vec<Value>),
    Table(HashMap<String, Value>),
    /// 闭包：模块内函数下标 + 已捕获上值。
    Closure { func: u32, upvalues: Vec<Value> },
}

/// 运行时值（栈与槽共用）。
///
/// - [`Value::Entity`]：不透明实体 ID，供脚本经原生调用桥接 ECS，**不**依赖 `spark-ecs`。
/// - [`Value::Func`]：模块内函数下标，供栈式调用与 JIT 特化识别。
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    /// 不透明实体 ID（与 `spark-ecs::Entity` 数值对应，由宿主约定）。
    Entity(u64),
    /// 模块函数下标。
    Func(u32),
    Handle(GcHandle),
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Self::Entity(id) => Some(*id as f64),
            _ => None,
        }
    }

    pub fn as_entity(&self) -> Option<u64> {
        match self {
            Self::Entity(id) => Some(*id),
            Self::Number(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_func(&self) -> Option<u32> {
        match self {
            Self::Func(i) => Some(*i),
            _ => None,
        }
    }

    pub fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::Entity(_) | Self::Func(_) | Self::Handle(_) => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Number(_) => "number",
            Self::Entity(_) => "entity",
            Self::Func(_) => "function",
            Self::Handle(_) => "object",
        }
    }
}

/// GC 结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum GcError {
    BadHandle(GcHandle),
}

impl GcError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadHandle(_) => "spark.gc.bad_handle",
        }
    }
}

impl fmt::Display for GcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for GcError {}

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
    /// 进程内累计分配次数（不因 GC 回落，供 VM 预算）。
    pub total_allocs: u64,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            allocs_since_gc: 0,
            gc_threshold: 256,
            total_allocs: 0,
        }
    }

    pub fn alloc(&mut self, obj: GcObject) -> GcHandle {
        self.allocs_since_gc += 1;
        self.total_allocs = self.total_allocs.saturating_add(1);
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

    #[test]
    fn entity_and_func_are_not_heap() {
        let e = Value::Entity(7);
        let f = Value::Func(3);
        assert_eq!(e.as_entity(), Some(7));
        assert_eq!(f.as_func(), Some(3));
        assert!(e.truthy());
    }
}
