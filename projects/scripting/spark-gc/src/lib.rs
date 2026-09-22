//! Spark GC：标记–清扫堆，供 `spark-vm` / `spark-script` 使用。
//!
//! 不变式：所有堆对象经 [`GcHandle`] 引用；根由宿主在 [`Heap::collect`] 前登记。
//! 栈值 [`Value`] 含非堆变体（数字、实体 ID、函数下标），GC 只追踪 [`Value::Handle`]。

#![forbid(missing_docs)]
use std::{collections::HashMap, fmt};

/// 堆对象句柄（分代可后续扩展；当前为槽位索引）。
///
/// `0..slots.len()` 为合法槽；无效或已清扫槽在 [`Heap::get`] 时返回 [`GcError::BadHandle`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcHandle(pub u32);

/// 堆上可回收对象。
///
/// 子引用仅出现在 [`Value::Handle`]；标记阶段从根沿这些句柄递归。
#[derive(Debug, Clone)]
pub enum GcObject {
    /// UTF-8 字符串载荷（无子句柄）。
    String(String),
    /// 有序数组；元素可为任意 [`Value`]（含嵌套句柄）。
    Array(Vec<Value>),
    /// 字符串键表；值侧可含句柄。
    Table(HashMap<String, Value>),
    /// 闭包：模块内函数下标 + 已捕获上值。
    Closure {
        /// 所属模块中的函数下标（与 [`Value::Func`] 同语义）。
        func: u32,
        /// 捕获的上值；其中的 [`Value::Handle`] 参与标记。
        upvalues: Vec<Value>,
    },
}

/// 运行时值（栈与槽共用）。
///
/// - [`Value::Entity`]：不透明实体 ID，供脚本经原生调用桥接 ECS，**不**依赖 `spark-ecs`。
/// - [`Value::Func`]：模块内函数下标，供栈式调用与 JIT 特化识别。
#[derive(Debug, Clone)]
pub enum Value {
    /// 空值；[`Self::truthy`] 为假。
    Null,
    /// 布尔；真值即自身。
    Bool(bool),
    /// IEEE-754 双精度；`0.0` / `NaN` 在 [`Self::truthy`] 中为假。
    Number(f64),
    /// 不透明实体 ID（与 `spark-ecs::Entity` 数值对应，由宿主约定）。
    Entity(u64),
    /// 模块函数下标。
    Func(u32),
    /// 指向堆对象的句柄；GC 根与子引用的唯一种类。
    Handle(GcHandle),
}

impl Value {
    /// 尝试转为数字：`Number` 原样；`Bool` → `0.0`/`1.0`；`Entity` → `id as f64`；其余 `None`。
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Self::Entity(id) => Some(*id as f64),
            _ => None,
        }
    }

    /// 尝试转为实体 ID：`Entity` 原样；非负有限整数 `Number` 截断为 `u64`；其余 `None`。
    pub fn as_entity(&self) -> Option<u64> {
        match self {
            Self::Entity(id) => Some(*id),
            Self::Number(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    /// 若为 [`Self::Func`] 则返回函数下标，否则 `None`。
    pub fn as_func(&self) -> Option<u32> {
        match self {
            Self::Func(i) => Some(*i),
            _ => None,
        }
    }

    /// 脚本真值：`Null`/`false`/`0.0`/`NaN` 为假；实体、函数、句柄恒为真。
    pub fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Number(n) => *n != 0.0 && !n.is_nan(),
            Self::Entity(_) | Self::Func(_) | Self::Handle(_) => true,
        }
    }

    /// 稳定类型名字符串（供诊断 / `typeof` 类原生）：`null`/`bool`/`number`/`entity`/`function`/`object`。
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
    /// 句柄越界、槽未分配或已被清扫（`live == false`）。
    BadHandle(GcHandle),
}

impl GcError {
    /// 稳定错误码，目前仅 `spark.gc.bad_handle`。
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
///
/// 空闲槽复用 `free` 列表；[`Self::collect`] 将未标记 `live` 槽回收并重置为占位字符串。
#[derive(Debug, Default)]
pub struct Heap {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// 自上次 GC 以来的分配次数；与 [`Self::gc_threshold`] 比较以触发 [`Self::maybe_collect`]。
    pub allocs_since_gc: usize,
    /// 触发自动收集的分配阈值（默认 `256`）；`0` 表示每次 `maybe_collect` 都收集。
    pub gc_threshold: usize,
    /// 进程内累计分配次数（不因 GC 回落，供 VM 预算）。
    pub total_allocs: u64,
}

impl Heap {
    /// 空堆：`gc_threshold = 256`，计数归零。
    pub fn new() -> Self {
        Self { slots: Vec::new(), free: Vec::new(), allocs_since_gc: 0, gc_threshold: 256, total_allocs: 0 }
    }

    /// 分配对象并返回句柄；优先复用 `free`，否则追加新槽。递增 [`Self::allocs_since_gc`] / [`Self::total_allocs`]。
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
        self.slots.push(Slot { obj, marked: false, live: true });
        GcHandle(idx)
    }

    /// 只读取对象；无效句柄 → [`GcError::BadHandle`]。
    pub fn get(&self, h: GcHandle) -> Result<&GcObject, GcError> {
        self.slots.get(h.0 as usize).filter(|s| s.live).map(|s| &s.obj).ok_or(GcError::BadHandle(h))
    }

    /// 可变取对象；无效句柄 → [`GcError::BadHandle`]。
    pub fn get_mut(&mut self, h: GcHandle) -> Result<&mut GcObject, GcError> {
        self.slots.get_mut(h.0 as usize).filter(|s| s.live).map(|s| &mut s.obj).ok_or(GcError::BadHandle(h))
    }

    /// 分配字符串对象并包装为 [`Value::Handle`]。
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
            let Some(slot) = self.slots.get_mut(h.0 as usize)
            else {
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
            }
            else {
                slot.live = false;
                slot.obj = GcObject::String(String::new());
                self.free.push(i as u32);
            }
        }
        self.allocs_since_gc = 0;
    }

    /// 若 [`Self::allocs_since_gc`] ≥ [`Self::gc_threshold`] 则执行 [`Self::collect`]。
    pub fn maybe_collect(&mut self, roots: &[Value]) {
        if self.allocs_since_gc >= self.gc_threshold {
            self.collect(roots);
        }
    }

    /// 当前 `live` 槽数量（不含已回收槽）。
    pub fn live_count(&self) -> usize {
        self.slots.iter().filter(|s| s.live).count()
    }
}
