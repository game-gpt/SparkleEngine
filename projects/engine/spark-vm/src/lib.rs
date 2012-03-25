//! Spark VM：栈式字节码解释器（无游戏语义）。

use std::collections::HashMap;

use spark_gc::{GcObject, Heap, Value};
use thiserror::Error;

/// 字节码操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    Nop = 0,
    LoadNull,
    LoadTrue,
    LoadFalse,
    /// 后跟 u16 常量下标。
    LoadConst,
    /// 后跟 u16 局部槽。
    LoadLocal,
    StoreLocal,
    /// 后跟 u16 全局名常量表下标（字符串）。
    LoadGlobal,
    StoreGlobal,
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    Not,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// 相对跳转 i16。
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    /// 参数个数 u8。
    Call,
    Return,
    /// 弹出 N 个（u8）。
    Pop,
    /// 打印栈顶（调试宿主钩子前的默认实现）。
    Print,
    /// JIT 入口占位：后跟 u32 stub id（由 spark-jit 填充）。
    JitEnter,
    /// 后跟 u16 字符串池下标；运行时分配到堆。
    LoadString,
    /// 后跟 u16 原生名下标 + u8 参数个数。
    CallNative,
}

/// 编译期函数。
#[derive(Debug, Clone)]
pub struct FuncProto {
    pub name: String,
    pub arity: u8,
    pub locals: u16,
    pub code: Vec<u8>,
    pub consts: Vec<Value>,
    /// 常量池中的字符串名（与 consts 并行，便于全局查找）。
    pub const_names: Vec<String>,
    /// 字符串字面量池。
    pub strings: Vec<String>,
}

impl FuncProto {
    pub fn new(name: impl Into<String>, arity: u8) -> Self {
        Self {
            name: name.into(),
            arity,
            locals: arity as u16,
            code: Vec::new(),
            consts: Vec::new(),
            const_names: Vec::new(),
            strings: Vec::new(),
        }
    }

    pub fn emit(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    pub fn emit_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    pub fn emit_u16(&mut self, v: u16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    pub fn emit_i16(&mut self, v: i16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    pub fn add_const_number(&mut self, n: f64) -> u16 {
        let i = self.consts.len() as u16;
        self.consts.push(Value::Number(n));
        self.const_names.push(String::new());
        i
    }

    pub fn add_const_name(&mut self, name: impl Into<String>) -> u16 {
        let name = name.into();
        let i = self.consts.len() as u16;
        self.consts.push(Value::Null);
        self.const_names.push(name);
        i
    }

    pub fn add_string(&mut self, s: impl Into<String>) -> u16 {
        let i = self.strings.len() as u16;
        self.strings.push(s.into());
        i
    }

    pub fn patch_i16(&mut self, at: usize, v: i16) {
        let b = v.to_le_bytes();
        self.code[at] = b[0];
        self.code[at + 1] = b[1];
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<FuncProto>,
    /// 入口函数下标。
    pub entry: usize,
    /// 原生函数名表（`CallNative` 索引）。
    pub native_names: Vec<String>,
}

impl Module {
    pub fn with_entry(functions: Vec<FuncProto>, entry: usize) -> Self {
        Self {
            functions,
            entry,
            native_names: Vec::new(),
        }
    }

    pub fn intern_native(&mut self, name: impl Into<String>) -> u16 {
        let name = name.into();
        if let Some(i) = self.native_names.iter().position(|n| n == &name) {
            return i as u16;
        }
        let i = self.native_names.len() as u16;
        self.native_names.push(name);
        i
    }
}

#[derive(Debug, Error)]
pub enum VmError {
    #[error("栈下溢")]
    StackUnderflow,
    #[error("字节码越界")]
    CodeOob,
    #[error("类型错误：期望 {expected}，得到 {got}")]
    TypeError { expected: &'static str, got: String },
    #[error("未知全局 `{0}`")]
    UnknownGlobal(String),
    #[error("调用栈溢出")]
    CallOverflow,
    #[error("返回栈异常")]
    BadReturn,
    #[error("除零")]
    DivByZero,
    #[error("未知原生函数 `{0}`")]
    UnknownNative(String),
    #[error("{0}")]
    Message(String),
}

/// 原生函数上下文。
pub struct NativeCtx<'a> {
    pub heap: &'a mut Heap,
    pub globals: &'a mut HashMap<String, Value>,
}

pub type NativeFn = Box<dyn FnMut(&mut NativeCtx<'_>, Vec<Value>) -> Result<Value, VmError>>;

#[derive(Debug)]
struct Frame {
    func: usize,
    ip: usize,
    /// 该帧在值栈上的基址（含参数）。
    stack_base: usize,
}

/// 宿主钩子（打印等）。
pub trait HostHooks {
    fn print(&mut self, text: &str);
}

pub struct StdHost;

impl HostHooks for StdHost {
    fn print(&mut self, text: &str) {
        println!("{text}");
    }
}

pub struct Vm {
    pub module: Module,
    pub heap: Heap,
    pub globals: HashMap<String, Value>,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    /// 每条函数热度（供 JIT）。
    pub hotness: Vec<u32>,
    pub natives: HashMap<String, NativeFn>,
}

impl Vm {
    pub fn new(module: Module) -> Self {
        let n = module.functions.len();
        Self {
            module,
            heap: Heap::new(),
            globals: HashMap::new(),
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            hotness: vec![0; n],
            natives: HashMap::new(),
        }
    }

    pub fn register_native<F>(&mut self, name: impl Into<String>, f: F)
    where
        F: FnMut(&mut NativeCtx<'_>, Vec<Value>) -> Result<Value, VmError> + 'static,
    {
        self.natives.insert(name.into(), Box::new(f));
    }

    pub fn run(&mut self, host: &mut dyn HostHooks) -> Result<Value, VmError> {
        let entry = self.module.entry;
        self.frames.clear();
        self.stack.clear();
        self.frames.push(Frame {
            func: entry,
            ip: 0,
            stack_base: 0,
        });
        // 为入口预留局部
        let locals = self.module.functions[entry].locals as usize;
        while self.stack.len() < locals {
            self.stack.push(Value::Null);
        }
        self.interpret(host)
    }

    fn read_u8(code: &[u8], ip: &mut usize) -> Result<u8, VmError> {
        let b = *code.get(*ip).ok_or(VmError::CodeOob)?;
        *ip += 1;
        Ok(b)
    }

    fn read_u16(code: &[u8], ip: &mut usize) -> Result<u16, VmError> {
        let a = Self::read_u8(code, ip)? as u16;
        let b = Self::read_u8(code, ip)? as u16;
        Ok(a | (b << 8))
    }

    fn read_i16(code: &[u8], ip: &mut usize) -> Result<i16, VmError> {
        Ok(Self::read_u16(code, ip)? as i16)
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value, VmError> {
        self.stack.last().ok_or(VmError::StackUnderflow)
    }

    fn bin_num(
        &mut self,
        op: impl Fn(f64, f64) -> Result<f64, VmError>,
    ) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let an = a
            .as_number()
            .ok_or_else(|| VmError::TypeError {
                expected: "number",
                got: a.type_name().into(),
            })?;
        let bn = b
            .as_number()
            .ok_or_else(|| VmError::TypeError {
                expected: "number",
                got: b.type_name().into(),
            })?;
        self.stack.push(Value::Number(op(an, bn)?));
        Ok(())
    }

    fn interpret(&mut self, host: &mut dyn HostHooks) -> Result<Value, VmError> {
        loop {
            if self.frames.is_empty() {
                return Ok(self.stack.pop().unwrap_or(Value::Null));
            }
            let fi = self.frames.len() - 1;
            let func_idx = self.frames[fi].func;
            self.hotness[func_idx] = self.hotness[func_idx].saturating_add(1);
            let ip = self.frames[fi].ip;
            let code = &self.module.functions[func_idx].code;
            if ip >= code.len() {
                // 隐式 return null
                let base = self.frames[fi].stack_base;
                self.frames.pop();
                self.stack.truncate(base);
                self.stack.push(Value::Null);
                continue;
            }
            let op = code[ip];
            self.frames[fi].ip = ip + 1;
            let op = match op {
                x if x == Op::Nop as u8 => Op::Nop,
                x if x == Op::LoadNull as u8 => Op::LoadNull,
                x if x == Op::LoadTrue as u8 => Op::LoadTrue,
                x if x == Op::LoadFalse as u8 => Op::LoadFalse,
                x if x == Op::LoadConst as u8 => Op::LoadConst,
                x if x == Op::LoadLocal as u8 => Op::LoadLocal,
                x if x == Op::StoreLocal as u8 => Op::StoreLocal,
                x if x == Op::LoadGlobal as u8 => Op::LoadGlobal,
                x if x == Op::StoreGlobal as u8 => Op::StoreGlobal,
                x if x == Op::Add as u8 => Op::Add,
                x if x == Op::Sub as u8 => Op::Sub,
                x if x == Op::Mul as u8 => Op::Mul,
                x if x == Op::Div as u8 => Op::Div,
                x if x == Op::Neg as u8 => Op::Neg,
                x if x == Op::Not as u8 => Op::Not,
                x if x == Op::Eq as u8 => Op::Eq,
                x if x == Op::Ne as u8 => Op::Ne,
                x if x == Op::Lt as u8 => Op::Lt,
                x if x == Op::Le as u8 => Op::Le,
                x if x == Op::Gt as u8 => Op::Gt,
                x if x == Op::Ge as u8 => Op::Ge,
                x if x == Op::Jump as u8 => Op::Jump,
                x if x == Op::JumpIfFalse as u8 => Op::JumpIfFalse,
                x if x == Op::JumpIfTrue as u8 => Op::JumpIfTrue,
                x if x == Op::Call as u8 => Op::Call,
                x if x == Op::Return as u8 => Op::Return,
                x if x == Op::Pop as u8 => Op::Pop,
                x if x == Op::Print as u8 => Op::Print,
                x if x == Op::JitEnter as u8 => Op::JitEnter,
                x if x == Op::LoadString as u8 => Op::LoadString,
                x if x == Op::CallNative as u8 => Op::CallNative,
                _ => {
                    return Err(VmError::Message(format!("未知操作码 {op}")));
                }
            };

            match op {
                Op::Nop => {}
                Op::LoadNull => self.stack.push(Value::Null),
                Op::LoadTrue => self.stack.push(Value::Bool(true)),
                Op::LoadFalse => self.stack.push(Value::Bool(false)),
                Op::LoadConst => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let c = self.module.functions[func_idx]
                        .consts
                        .get(idx as usize)
                        .cloned()
                        .ok_or(VmError::CodeOob)?;
                    self.stack.push(c);
                }
                Op::LoadLocal => {
                    let mut ip = self.frames[fi].ip;
                    let slot = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let base = self.frames[fi].stack_base;
                    let v = self
                        .stack
                        .get(base + slot as usize)
                        .cloned()
                        .unwrap_or(Value::Null);
                    self.stack.push(v);
                }
                Op::StoreLocal => {
                    let mut ip = self.frames[fi].ip;
                    let slot = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let v = self.pop()?;
                    let base = self.frames[fi].stack_base;
                    let i = base + slot as usize;
                    if i >= self.stack.len() {
                        self.stack.resize(i + 1, Value::Null);
                    }
                    self.stack[i] = v;
                }
                Op::LoadGlobal => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let name = self.module.functions[func_idx]
                        .const_names
                        .get(idx as usize)
                        .cloned()
                        .unwrap_or_default();
                    let v = self
                        .globals
                        .get(&name)
                        .cloned()
                        .ok_or(VmError::UnknownGlobal(name))?;
                    self.stack.push(v);
                }
                Op::StoreGlobal => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let name = self.module.functions[func_idx]
                        .const_names
                        .get(idx as usize)
                        .cloned()
                        .unwrap_or_default();
                    let v = self.pop()?;
                    self.globals.insert(name, v);
                }
                Op::Add => {
                    // 数字或字符串拼接
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (&a, &b) {
                        (Value::Number(x), Value::Number(y)) => {
                            self.stack.push(Value::Number(x + y));
                        }
                        _ => {
                            let sa = self.value_to_string(&a)?;
                            let sb = self.value_to_string(&b)?;
                            let v = self.heap.alloc_string(format!("{sa}{sb}"));
                            self.stack.push(v);
                        }
                    }
                }
                Op::Sub => self.bin_num(|a, b| Ok(a - b))?,
                Op::Mul => self.bin_num(|a, b| Ok(a * b))?,
                Op::Div => self.bin_num(|a, b| {
                    if b == 0.0 {
                        Err(VmError::DivByZero)
                    } else {
                        Ok(a / b)
                    }
                })?,
                Op::Neg => {
                    let a = self.pop()?;
                    let n = a.as_number().ok_or_else(|| VmError::TypeError {
                        expected: "number",
                        got: a.type_name().into(),
                    })?;
                    self.stack.push(Value::Number(-n));
                }
                Op::Not => {
                    let a = self.pop()?;
                    self.stack.push(Value::Bool(!a.truthy()));
                }
                Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let r = match op {
                        Op::Eq => values_eq(&a, &b),
                        Op::Ne => !values_eq(&a, &b),
                        _ => {
                            let an = a.as_number().ok_or_else(|| VmError::TypeError {
                                expected: "number",
                                got: a.type_name().into(),
                            })?;
                            let bn = b.as_number().ok_or_else(|| VmError::TypeError {
                                expected: "number",
                                got: b.type_name().into(),
                            })?;
                            match op {
                                Op::Lt => an < bn,
                                Op::Le => an <= bn,
                                Op::Gt => an > bn,
                                Op::Ge => an >= bn,
                                _ => unreachable!(),
                            }
                        }
                    };
                    self.stack.push(Value::Bool(r));
                }
                Op::Jump => {
                    let mut ip = self.frames[fi].ip;
                    let off = Self::read_i16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ((ip as isize) + off as isize) as usize;
                }
                Op::JumpIfFalse => {
                    let mut ip = self.frames[fi].ip;
                    let off = Self::read_i16(&self.module.functions[func_idx].code, &mut ip)?;
                    let cond = self.pop()?;
                    if !cond.truthy() {
                        self.frames[fi].ip = ((ip as isize) + off as isize) as usize;
                    } else {
                        self.frames[fi].ip = ip;
                    }
                }
                Op::JumpIfTrue => {
                    let mut ip = self.frames[fi].ip;
                    let off = Self::read_i16(&self.module.functions[func_idx].code, &mut ip)?;
                    let cond = self.pop()?;
                    if cond.truthy() {
                        self.frames[fi].ip = ((ip as isize) + off as isize) as usize;
                    } else {
                        self.frames[fi].ip = ip;
                    }
                }
                Op::Call => {
                    let mut ip = self.frames[fi].ip;
                    let argc = Self::read_u8(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    // 栈：... callee, arg0..argN-1
                    if self.stack.len() < argc as usize + 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    let callee_idx = self.stack.len() - argc as usize - 1;
                    let callee = self.stack[callee_idx].clone();
                    let Value::Number(fidx) = callee else {
                        return Err(VmError::TypeError {
                            expected: "function-index",
                            got: callee.type_name().into(),
                        });
                    };
                    let fidx = fidx as usize;
                    if fidx >= self.module.functions.len() {
                        return Err(VmError::CodeOob);
                    }
                    let arity = self.module.functions[fidx].arity;
                    if argc != arity {
                        return Err(VmError::Message(format!(
                            "参数个数不符：期望 {arity}，得到 {argc}"
                        )));
                    }
                    if self.frames.len() > 256 {
                        return Err(VmError::CallOverflow);
                    }
                    // 去掉 callee，参数留在栈上作为局部 0..arity
                    self.stack.remove(callee_idx);
                    let base = self.stack.len() - arity as usize;
                    let need = self.module.functions[fidx].locals as usize;
                    while self.stack.len() < base + need {
                        self.stack.push(Value::Null);
                    }
                    self.frames.push(Frame {
                        func: fidx,
                        ip: 0,
                        stack_base: base,
                    });
                }
                Op::Return => {
                    let ret = self.pop().unwrap_or(Value::Null);
                    let frame = self.frames.pop().ok_or(VmError::BadReturn)?;
                    self.stack.truncate(frame.stack_base);
                    self.stack.push(ret);
                    if self.frames.is_empty() {
                        return Ok(self.stack.pop().unwrap_or(Value::Null));
                    }
                }
                Op::Pop => {
                    let mut ip = self.frames[fi].ip;
                    let n = Self::read_u8(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    for _ in 0..n {
                        let _ = self.pop()?;
                    }
                }
                Op::Print => {
                    let v = self.peek()?.clone();
                    let s = self.value_to_string(&v)?;
                    host.print(&s);
                }
                Op::JitEnter => {
                    // 跳过 stub id，解释器忽略（JIT 接管前为 no-op）
                    let mut ip = self.frames[fi].ip;
                    let _ = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    let _ = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                }
                Op::LoadString => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let s = self.module.functions[func_idx]
                        .strings
                        .get(idx as usize)
                        .cloned()
                        .ok_or(VmError::CodeOob)?;
                    let v = self.heap.alloc_string(s);
                    self.stack.push(v);
                }
                Op::CallNative => {
                    let mut ip = self.frames[fi].ip;
                    let name_idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    let argc = Self::read_u8(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let name = self
                        .module
                        .native_names
                        .get(name_idx as usize)
                        .cloned()
                        .ok_or(VmError::CodeOob)?;
                    if self.stack.len() < argc as usize {
                        return Err(VmError::StackUnderflow);
                    }
                    let args: Vec<Value> = self
                        .stack
                        .drain(self.stack.len() - argc as usize..)
                        .collect();
                    let mut native = self
                        .natives
                        .remove(&name)
                        .ok_or_else(|| VmError::UnknownNative(name.clone()))?;
                    let result = {
                        let mut ctx = NativeCtx {
                            heap: &mut self.heap,
                            globals: &mut self.globals,
                        };
                        native(&mut ctx, args)
                    };
                    self.natives.insert(name, native);
                    self.stack.push(result?);
                }
            }

            // 周期性 GC：栈 + 全局作根
            if self.heap.allocs_since_gc >= self.heap.gc_threshold {
                let mut roots = self.stack.clone();
                roots.extend(self.globals.values().cloned());
                self.heap.collect(&roots);
            }
        }
    }

    pub fn value_to_string(&self, v: &Value) -> Result<String, VmError> {
        Ok(match v {
            Value::Null => "null".into(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if *n == n.trunc() && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
            Value::Handle(h) => match self.heap.get(*h) {
                Ok(GcObject::String(s)) => s.clone(),
                Ok(_) => format!("<object {}>", h.0),
                Err(_) => "<dangling>".into(),
            },
        })
    }
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Handle(x), Value::Handle(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BufHost(String);
    impl HostHooks for BufHost {
        fn print(&mut self, text: &str) {
            self.0.push_str(text);
            self.0.push('\n');
        }
    }

    #[test]
    fn add_and_return() {
        let mut f = FuncProto::new("main", 0);
        let c1 = f.add_const_number(40.0);
        let c2 = f.add_const_number(2.0);
        f.emit(Op::LoadConst);
        f.emit_u16(c1);
        f.emit(Op::LoadConst);
        f.emit_u16(c2);
        f.emit(Op::Add);
        f.emit(Op::Return);
        let mut vm = Vm::new(Module {
            functions: vec![f],
            entry: 0,
            native_names: Vec::new(),
        });
        let mut host = BufHost(String::new());
        let v = vm.run(&mut host).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }
}
