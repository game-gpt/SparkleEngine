//! Spark VM：栈式字节码解释器（无游戏语义）。
//!
//! # 与 ECS / JIT 的边界
//!
//! - **ECS**：世界与 Component 在 `spark-ecs`；脚本经 [`Value::Entity`] 与 [`CallNative`]
//!   交换不透明 ID。调度侧用 [`Vm::call_function`] 调脚本 `micro`，勿把 World 塞进 VM。
//! - **JIT**：每帧解释累加 [`Vm::hotness`]；`spark-jit` 对热点 [`FuncProto`] 做字节码特化，
//!   [`Op::JitEnter`] 预留原生 stub 槽（解释路径跳过）。
//! - **栈式**：操作数在值栈，调用帧只记 `func` / `ip` / `stack_base`，利于特化与调试。

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use spark_diagnostics::{Error, ErrorArg, ErrorArgs, ErrorCode};
use spark_gc::{GcObject, Heap, Value};

mod verify;

pub use verify::{verify_bytecode, BytecodeVerifyError};

/// VM 结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum VmError {
    StackUnderflow,
    CodeOob,
    TypeError {
        expected: &'static str,
        got: String,
    },
    UnknownGlobal(String),
    UnknownFunction(String),
    CallOverflow,
    BadReturn,
    DivByZero,
    UnknownNative(String),
    UnknownOpcode(u8),
    ArityMismatch { expected: u16, got: u16 },
    BadNativeArg { name: &'static str },
    /// 堆句柄无效。
    BadHandle,
}

impl VmError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::StackUnderflow => "spark.vm.stack_underflow",
            Self::CodeOob => "spark.vm.code_oob",
            Self::TypeError { .. } => "spark.vm.type_mismatch",
            Self::UnknownGlobal(_) => "spark.vm.unknown_global",
            Self::UnknownFunction(_) => "spark.vm.unknown_function",
            Self::CallOverflow => "spark.vm.call_overflow",
            Self::BadReturn => "spark.vm.bad_return",
            Self::DivByZero => "spark.vm.div_by_zero",
            Self::UnknownNative(_) => "spark.vm.unknown_native",
            Self::UnknownOpcode(_) => "spark.vm.unknown_opcode",
            Self::ArityMismatch { .. } => "spark.vm.arity_mismatch",
            Self::BadNativeArg { .. } => "spark.vm.bad_native_arg",
            Self::BadHandle => "spark.vm.bad_handle",
        }
    }

    /// 类型化参数（供 Diagnostic / LogEvent / localization 渲染）。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::TypeError { expected, got } => ErrorArgs::new()
                .with("expected", ErrorArg::TypeName(Arc::from(*expected)))
                .with("got", ErrorArg::TypeName(Arc::from(got.as_str()))),
            Self::UnknownGlobal(name)
            | Self::UnknownFunction(name)
            | Self::UnknownNative(name) => {
                ErrorArgs::new().with("name", ErrorArg::String(Arc::from(name.as_str())))
            }
            Self::UnknownOpcode(op) => ErrorArgs::new().with("opcode", ErrorArg::Opcode(*op)),
            Self::ArityMismatch { expected, got } => ErrorArgs::new()
                .with("expected", ErrorArg::Unsigned(u64::from(*expected)))
                .with("got", ErrorArg::Unsigned(u64::from(*got))),
            Self::BadNativeArg { name } => {
                ErrorArgs::new().with("name", ErrorArg::String(Arc::from(*name)))
            }
            Self::StackUnderflow
            | Self::CodeOob
            | Self::CallOverflow
            | Self::BadReturn
            | Self::DivByZero
            | Self::BadHandle => ErrorArgs::new(),
        }
    }

    pub fn to_error(&self) -> Error {
        Error::new(ErrorCode::parse(self.code())).with_args(self.args())
    }
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for VmError {}

/// 字节码操作（操作数小端紧随操作码）。
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
    /// 相对跳转 i16（相对操作数之后）。
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    /// 参数个数 u8；栈顶为 argN-1…arg0，其下为 callee（[`Value::Func`]）。
    Call,
    Return,
    /// 弹出 N 个（u8）。
    Pop,
    /// 打印栈顶（不弹出）。
    Print,
    /// JIT 入口占位：后跟 u32 stub id。
    JitEnter,
    /// 后跟 u16 字符串池下标；运行时分配到堆。
    LoadString,
    /// 后跟 u16 原生名下标 + u8 参数个数。
    CallNative,
    /// 取模。
    Mod,
    /// 实例方法派发：后跟 u16 方法名字符串池下标 + u8 参数个数（不含接收者）。
    /// 栈：`recv, arg0…argN-1`。按 `recv.__class` + 方法名查找 `Class_method`。
    Send,
    /// 读表字段：后跟 u16 字符串池下标。弹出对象，压入字段值。
    GetField,
    /// 写表字段：后跟 u16 字符串池下标。弹出值再弹出对象，写入后压回值。
    SetField,
    /// 复制栈顶。
    Dup,
    /// 新建空表：无操作数，压入 `Table` 句柄。
    NewTable,
    /// 新建数组：后跟 u8 元素个数；弹出 N 个元素（底→顶为 0..N-1），压入 `Array`。
    NewArray,
    /// 宿主槽位调用：后跟 u16 槽位 + u8 参数个数（链接后 ABI；不经字符串查找）。
    CallHost,
}

/// 编译期函数原型（解释与 JIT 共用）。
#[derive(Debug, Clone)]
pub struct FuncProto {
    pub name: String,
    pub arity: u8,
    pub locals: u16,
    pub code: Vec<u8>,
    pub consts: Vec<Value>,
    /// 与 `consts` 并行的名字槽（全局符号用）。
    pub const_names: Vec<String>,
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

    pub fn add_const_func(&mut self, func: u32) -> u16 {
        let i = self.consts.len() as u16;
        self.consts.push(Value::Func(func));
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
    /// 入口函数下标（通常为隐式 `__main`）。
    pub entry: usize,
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

    pub fn find_function(&self, name: &str) -> Option<usize> {
        self.functions.iter().position(|f| f.name == name)
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

    /// 把多个脚本模块的方法链进同一命名空间（同名后者覆盖）；不含各脚本 `__main`。
    pub fn link_methods(modules: &[Module]) -> Module {
        let mut name_to_idx: HashMap<String, usize> = HashMap::new();
        let mut functions: Vec<FuncProto> = Vec::new();
        let mut native_names: Vec<String> = Vec::new();

        for module in modules {
            for name in &module.native_names {
                if !native_names.iter().any(|n| n == name) {
                    native_names.push(name.clone());
                }
            }
            for func in &module.functions {
                if func.name == "__main" {
                    continue;
                }
                if !name_to_idx.contains_key(&func.name) {
                    name_to_idx.insert(func.name.clone(), functions.len());
                    functions.push(FuncProto::new(func.name.clone(), func.arity));
                }
            }
        }

        let native_to_idx: HashMap<String, u16> = native_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u16))
            .collect();

        for module in modules {
            for func in &module.functions {
                if func.name == "__main" {
                    continue;
                }
                let idx = name_to_idx[&func.name];
                functions[idx] =
                    remap_func_against(module, func, &name_to_idx, &native_to_idx);
            }
        }

        let entry = functions.len();
        functions.push(FuncProto::new("__main", 0));
        functions[entry].emit(Op::LoadNull);
        functions[entry].emit(Op::Return);

        Module {
            functions,
            entry,
            native_names,
        }
    }
}

/// 按「旧模块下标 → 函数名 → 新模块下标」重写 `Value::Func`。
/// `CallNative` 名走函数字符串池，链接后不改操作数。
fn remap_func_against(
    old_module: &Module,
    func: &FuncProto,
    name_to_idx: &HashMap<String, usize>,
    _native_to_idx: &HashMap<String, u16>,
) -> FuncProto {
    let mut out = func.clone();
    for c in &mut out.consts {
        if let Value::Func(old_idx) = c {
            if let Some(old_f) = old_module.functions.get(*old_idx as usize) {
                if let Some(&new_idx) = name_to_idx.get(&old_f.name) {
                    *c = Value::Func(new_idx as u32);
                }
            }
        }
    }
    out
}

/// 原生函数上下文（宿主可经此访问堆与全局；ECS World 由闭包捕获）。
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

/// 宿主钩子（打印等；ECS 侧可换实现）。
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
    /// 每条函数解释步热度（供 JIT）。
    pub hotness: Vec<u32>,
    pub natives: HashMap<String, NativeFn>,
    /// 宿主槽位 → 短名（与编译期 `HostSchema` 插入顺序一致）。
    pub host_slot_names: Vec<String>,
    /// 单次 `interpret` 步数上限（RGSS 宿主可调）。
    pub step_limit: u64,
    /// CallNative / Send / CallHost 调用计数（诊断用）。
    pub call_hits: HashMap<String, u32>,
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
            host_slot_names: Vec::new(),
            step_limit: 5_000_000,
            call_hits: HashMap::new(),
        }
    }

    /// 装载编译期宿主槽位表（短名顺序 = 槽位下标）。
    pub fn prepare_host_slots(&mut self, names: impl IntoIterator<Item = impl Into<String>>) {
        self.host_slot_names = names.into_iter().map(Into::into).collect();
    }

    pub fn register_native<F>(&mut self, name: impl Into<String>, f: F)
    where
        F: FnMut(&mut NativeCtx<'_>, Vec<Value>) -> Result<Value, VmError> + 'static,
    {
        self.natives.insert(name.into(), Box::new(f));
    }

    /// 从模块入口运行（脚本顶层 / `__main`）。
    pub fn run(&mut self, host: &mut dyn HostHooks) -> Result<Value, VmError> {
        let entry = self.module.entry;
        self.call_index(entry, &[], host)
    }

    /// 在已链接方法表上执行某一脚本的 `__main`（重写其函数下标）。
    pub fn run_script_main(
        &mut self,
        script: &Module,
        host: &mut dyn HostHooks,
    ) -> Result<Value, VmError> {
        let main = script
            .functions
            .get(script.entry)
            .ok_or(VmError::CodeOob)?;
        let name_to_idx: HashMap<String, usize> = self
            .module
            .functions
            .iter()
            .enumerate()
            .map(|(i, f)| (f.name.clone(), i))
            .collect();
        let native_to_idx: HashMap<String, u16> = self
            .module
            .native_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u16))
            .collect();
        // 脚本里有、链接表没有的 native：补进模块名表。
        for name in &script.native_names {
            if !native_to_idx.contains_key(name) {
                self.module.intern_native(name.clone());
            }
        }
        let remapped = remap_func_against(script, main, &name_to_idx, &HashMap::new());
        let idx = self.module.functions.len();
        self.module.functions.push(remapped);
        self.hotness.push(0);
        let result = self.call_index(idx, &[], host);
        self.module.functions.pop();
        self.hotness.pop();
        result
    }

    /// 按名调用脚本函数（ECS System / 事件回调入口）。
    pub fn call_function(
        &mut self,
        name: &str,
        args: &[Value],
        host: &mut dyn HostHooks,
    ) -> Result<Value, VmError> {
        let idx = self
            .module
            .find_function(name)
            .ok_or_else(|| VmError::UnknownFunction(name.into()))?;
        self.call_index(idx, args, host)
    }

    fn invoke_native(&mut self, name: &str, args: Vec<Value>) -> Result<Value, VmError> {
        if !self.natives.contains_key(name) {
            // 未注册 native 默认空实现，便于逐步补齐 RGSS API。
            self.natives.insert(
                name.to_string(),
                Box::new(|_ctx, _args| Ok(Value::Null)),
            );
        }
        let mut native = self
            .natives
            .remove(name)
            .ok_or_else(|| VmError::UnknownNative(name.to_string()))?;
        let result = {
            let mut ctx = NativeCtx {
                heap: &mut self.heap,
                globals: &mut self.globals,
            };
            native(&mut ctx, args)
        };
        self.natives.insert(name.to_string(), native);
        result
    }

    fn call_index(
        &mut self,
        func: usize,
        args: &[Value],
        host: &mut dyn HostHooks,
    ) -> Result<Value, VmError> {
        let arity = self.module.functions[func].arity as usize;
        if args.len() != arity {
            return Err(VmError::ArityMismatch {
                expected: arity as u16,
                got: args.len() as u16,
            });
        }
        self.frames.clear();
        self.stack.clear();
        self.stack.extend_from_slice(args);
        let need = self.module.functions[func].locals as usize;
        while self.stack.len() < need {
            self.stack.push(Value::Null);
        }
        self.frames.push(Frame {
            func,
            ip: 0,
            stack_base: 0,
        });
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
        // RGSS 宿主阶段：非数字（含 null / 对象 stub）按 0 参与算术。
        let an = a.as_number().unwrap_or(0.0);
        let bn = b.as_number().unwrap_or(0.0);
        self.stack.push(Value::Number(op(an, bn)?));
        Ok(())
    }

    fn interpret(&mut self, host: &mut dyn HostHooks) -> Result<Value, VmError> {
        let mut steps: u64 = 0;
        let step_limit = self.step_limit;
        loop {
            steps += 1;
            if steps > step_limit {
                let mut best = 0u32;
                let mut best_name = "?".to_string();
                for (i, h) in self.hotness.iter().enumerate() {
                    if *h >= best {
                        best = *h;
                        best_name = self
                            .module
                            .functions
                            .get(i)
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| "?".into());
                    }
                }
                eprintln!("rgss play: step limit hit hot={best_name}({best}) steps={steps}");
                if !self.call_hits.is_empty() {
                    let mut hits: Vec<_> = self.call_hits.iter().collect();
                    hits.sort_by(|a, b| b.1.cmp(a.1));
                    for (name, n) in hits.into_iter().take(8) {
                        eprintln!("rgss play: call_hit {name}={n}");
                    }
                }
                return Err(VmError::CallOverflow);
            }
            if self.frames.is_empty() {
                return Ok(self.stack.pop().unwrap_or(Value::Null));
            }
            let fi = self.frames.len() - 1;
            let func_idx = self.frames[fi].func;
            self.hotness[func_idx] = self.hotness[func_idx].saturating_add(1);
            let ip = self.frames[fi].ip;
            let code = &self.module.functions[func_idx].code;
            if ip >= code.len() {
                let base = self.frames[fi].stack_base;
                self.frames.pop();
                self.stack.truncate(base);
                self.stack.push(Value::Null);
                continue;
            }
            let op_byte = code[ip];
            self.frames[fi].ip = ip + 1;
            let op = decode_op(op_byte).ok_or_else(|| {
                VmError::UnknownOpcode(op_byte)
            })?;

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
                    // Ruby 未定义全局读为 nil；常量未定义暂同此处理，便于 stub 宿主。
                    let v = self.globals.get(&name).cloned().unwrap_or(Value::Null);
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
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (&a, &b) {
                        (Value::Number(x), Value::Number(y)) => {
                            self.stack.push(Value::Number(x + y));
                        }
                        (Value::Handle(ha), Value::Handle(hb)) => {
                            match (self.heap.get(*ha), self.heap.get(*hb)) {
                                (Ok(GcObject::Array(aa)), Ok(GcObject::Array(bb))) => {
                                    let mut out = aa.clone();
                                    out.extend(bb.iter().cloned());
                                    let h = self.heap.alloc(GcObject::Array(out));
                                    self.stack.push(Value::Handle(h));
                                }
                                _ => {
                                    let sa = self.value_to_string(&a)?;
                                    let sb = self.value_to_string(&b)?;
                                    let v = self.heap.alloc_string(format!("{sa}{sb}"));
                                    self.stack.push(v);
                                }
                            }
                        }
                        _ => {
                            let sa = self.value_to_string(&a)?;
                            let sb = self.value_to_string(&b)?;
                            let v = self.heap.alloc_string(format!("{sa}{sb}"));
                            self.stack.push(v);
                        }
                    }
                }
                Op::Mul => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    match (&a, &b) {
                        (Value::Handle(h), Value::Number(n)) => {
                            match self.heap.get(*h) {
                                Ok(GcObject::Array(arr)) => {
                                    let times = (*n).max(0.0) as usize;
                                    let mut out = Vec::with_capacity(arr.len() * times);
                                    for _ in 0..times {
                                        out.extend(arr.iter().cloned());
                                    }
                                    let nh = self.heap.alloc(GcObject::Array(out));
                                    self.stack.push(Value::Handle(nh));
                                }
                                Ok(GcObject::String(s)) => {
                                    let times = (*n).max(0.0) as usize;
                                    let out = s.repeat(times);
                                    self.stack.push(self.heap.alloc_string(out));
                                }
                                _ => {
                                    let an = a.as_number().unwrap_or(0.0);
                                    let bn = b.as_number().unwrap_or(0.0);
                                    self.stack.push(Value::Number(an * bn));
                                }
                            }
                        }
                        _ => {
                            let an = a.as_number().unwrap_or(0.0);
                            let bn = b.as_number().unwrap_or(0.0);
                            self.stack.push(Value::Number(an * bn));
                        }
                    }
                }
                Op::Sub => self.bin_num(|a, b| Ok(a - b))?,
                Op::Div => self.bin_num(|a, b| {
                    if b == 0.0 {
                        Ok(0.0)
                    } else {
                        Ok(a / b)
                    }
                })?,
                Op::Mod => self.bin_num(|a, b| {
                    if b == 0.0 {
                        Ok(0.0)
                    } else {
                        Ok(a % b)
                    }
                })?,
                Op::Neg => {
                    let a = self.pop()?;
                    let n = a.as_number().unwrap_or(0.0);
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
                            let an = a.as_number().unwrap_or(0.0);
                            let bn = b.as_number().unwrap_or(0.0);
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
                    if self.stack.len() < argc as usize + 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    let callee_idx = self.stack.len() - argc as usize - 1;
                    let callee = self.stack[callee_idx].clone();
                    let fidx = callee.as_func().ok_or_else(|| VmError::TypeError {
                        expected: "function",
                        got: callee.type_name().into(),
                    })? as usize;
                    if fidx >= self.module.functions.len() {
                        return Err(VmError::CodeOob);
                    }
                    let arity = self.module.functions[fidx].arity;
                    let mut argc = argc;
                    if argc != arity {
                        if arity > argc {
                            // 缺参垫 nil（默认参数 / `Foo.bar` 缺 self 等）。
                            let need = (arity - argc) as usize;
                            let insert_at = callee_idx + 1 + argc as usize;
                            for _ in 0..need {
                                self.stack.insert(insert_at, Value::Null);
                            }
                            argc = arity;
                        } else {
                            let drop_n = (argc - arity) as usize;
                            let start = self.stack.len() - drop_n;
                            self.stack.truncate(start);
                            argc = arity;
                        }
                    }
                    if self.frames.len() > 256 {
                        let stack: Vec<_> = self
                            .frames
                            .iter()
                            .rev()
                            .take(10)
                            .map(|f| {
                                self.module
                                    .functions
                                    .get(f.func)
                                    .map(|p| p.name.as_str())
                                    .unwrap_or("?")
                            })
                            .collect();
                        eprintln!(
                            "rgss play: call depth overflow top={}",
                            stack.join(" <- ")
                        );
                        return Err(VmError::CallOverflow);
                    }
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
                    // CallNative 操作数 = 本函数字符串池下标（脚本前端）；旧字节码回退 native_names。
                    let name = self.module.functions[func_idx]
                        .strings
                        .get(name_idx as usize)
                        .cloned()
                        .or_else(|| self.module.native_names.get(name_idx as usize).cloned())
                        .ok_or(VmError::CodeOob)?;
                    if self.stack.len() < argc as usize {
                        return Err(VmError::StackUnderflow);
                    }
                    let args: Vec<Value> = self
                        .stack
                        .drain(self.stack.len() - argc as usize..)
                        .collect();
                    *self.call_hits.entry(format!("native:{name}")).or_insert(0) += 1;
                    let result = self.invoke_native(&name, args)?;
                    self.stack.push(result);
                }
                Op::CallHost => {
                    let mut ip = self.frames[fi].ip;
                    let slot = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    let argc = Self::read_u8(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let name = self
                        .host_slot_names
                        .get(slot as usize)
                        .cloned()
                        .ok_or_else(|| VmError::UnknownNative(format!("host_slot:{slot}")))?;
                    if self.stack.len() < argc as usize {
                        return Err(VmError::StackUnderflow);
                    }
                    let args: Vec<Value> = self
                        .stack
                        .drain(self.stack.len() - argc as usize..)
                        .collect();
                    *self.call_hits.entry(format!("host:{slot}:{name}")).or_insert(0) += 1;
                    let result = self.invoke_native(&name, args)?;
                    self.stack.push(result);
                }
                Op::Send => {
                    let mut ip = self.frames[fi].ip;
                    let method_idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    let argc = Self::read_u8(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let method = self.module.functions[func_idx]
                        .strings
                        .get(method_idx as usize)
                        .cloned()
                        .ok_or(VmError::CodeOob)?;
                    if self.stack.len() < argc as usize + 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    let args: Vec<Value> = self
                        .stack
                        .drain(self.stack.len() - argc as usize..)
                        .collect();
                    let recv = self.pop()?;
                    // 数组内建：size / [] / []=
                    if let Value::Handle(h) = &recv {
                        if let Ok(GcObject::Array(arr)) = self.heap.get(*h) {
                            match method.as_str() {
                                "size" | "length" => {
                                    self.stack.push(Value::Number(arr.len() as f64));
                                    continue;
                                }
                                "push" | "<<" => {
                                    let val = args.first().cloned().unwrap_or(Value::Null);
                                    if let Ok(GcObject::Array(arr)) = self.heap.get_mut(*h) {
                                        arr.push(val.clone());
                                    }
                                    self.stack.push(recv);
                                    continue;
                                }
                                "sum" => {
                                    let mut total = 0.0;
                                    for v in arr {
                                        total += v.as_number().unwrap_or(0.0);
                                    }
                                    self.stack.push(Value::Number(total));
                                    continue;
                                }
                                "min" => {
                                    let mut best: Option<f64> = None;
                                    for v in arr {
                                        if let Some(n) = v.as_number() {
                                            best = Some(best.map_or(n, |b| b.min(n)));
                                        }
                                    }
                                    self.stack.push(best.map(Value::Number).unwrap_or(Value::Null));
                                    continue;
                                }
                                "max" => {
                                    let mut best: Option<f64> = None;
                                    for v in arr {
                                        if let Some(n) = v.as_number() {
                                            best = Some(best.map_or(n, |b| b.max(n)));
                                        }
                                    }
                                    self.stack.push(best.map(Value::Number).unwrap_or(Value::Null));
                                    continue;
                                }
                                "[]" if argc == 1 => {
                                    let idx = args.first().and_then(|v| v.as_number()).unwrap_or(0.0) as isize;
                                    let i = if idx < 0 {
                                        (arr.len() as isize + idx) as usize
                                    } else {
                                        idx as usize
                                    };
                                    let v = arr.get(i).cloned().unwrap_or(Value::Null);
                                    self.stack.push(v);
                                    continue;
                                }
                                "[]" if argc == 2 => {
                                    // `arr[start, length]` 切片。
                                    let start = args.first().and_then(|v| v.as_number()).unwrap_or(0.0) as isize;
                                    let len = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0).max(0.0) as usize;
                                    let start = if start < 0 {
                                        (arr.len() as isize + start).max(0) as usize
                                    } else {
                                        start as usize
                                    };
                                    let end = (start + len).min(arr.len());
                                    let slice = if start < arr.len() {
                                        arr[start..end].to_vec()
                                    } else {
                                        Vec::new()
                                    };
                                    let nh = self.heap.alloc(GcObject::Array(slice));
                                    self.stack.push(Value::Handle(nh));
                                    continue;
                                }
                                "[]=" if argc == 2 => {
                                    let idx = args.first().and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
                                    let val = args.get(1).cloned().unwrap_or(Value::Null);
                                    if let Ok(GcObject::Array(arr)) = self.heap.get_mut(*h) {
                                        if idx >= arr.len() {
                                            arr.resize(idx + 1, Value::Null);
                                        }
                                        arr[idx] = val.clone();
                                    }
                                    self.stack.push(val);
                                    continue;
                                }
                                _ => {}
                            }
                        }
                    }
                    let class_name = match &recv {
                        Value::Handle(h) => match self.heap.get(*h) {
                            Ok(GcObject::Table(map)) => map
                                .get("__class")
                                .and_then(|v| match v {
                                    Value::Handle(ch) => match self.heap.get(*ch) {
                                        Ok(GcObject::String(s)) => Some(s.clone()),
                                        _ => None,
                                    },
                                    _ => None,
                                })
                                .unwrap_or_else(|| "Object".into()),
                            _ => "Object".into(),
                        },
                        _ => "Object".into(),
                    };
                    let fname = format!("{class_name}_{method}");
                    *self.call_hits.entry(format!("send:{fname}")).or_insert(0) += 1;
                    if let Some(fidx) = self.module.find_function(&fname) {
                        let arity = self.module.functions[fidx].arity;
                        let mut call_args: Vec<Value> = Vec::with_capacity(args.len() + 1);
                        call_args.push(recv);
                        call_args.extend(args);
                        // 与 Call 一致：缺参垫 nil，多余实参丢弃。
                        let arity_usize = arity as usize;
                        if call_args.len() < arity_usize {
                            call_args.resize(arity_usize, Value::Null);
                        } else if call_args.len() > arity_usize {
                            call_args.truncate(arity_usize);
                        }
                        if self.frames.len() > 256 {
                            let stack: Vec<_> = self
                                .frames
                                .iter()
                                .rev()
                                .take(10)
                                .map(|f| {
                                    self.module
                                        .functions
                                        .get(f.func)
                                        .map(|p| p.name.as_str())
                                        .unwrap_or("?")
                                })
                                .collect();
                            eprintln!(
                                "rgss play: send depth overflow top={}",
                                stack.join(" <- ")
                            );
                            return Err(VmError::CallOverflow);
                        }
                        let base = self.stack.len();
                        self.stack.extend(call_args);
                        let need = self.module.functions[fidx].locals as usize;
                        while self.stack.len() < base + need {
                            self.stack.push(Value::Null);
                        }
                        self.frames.push(Frame {
                            func: fidx,
                            ip: 0,
                            stack_base: base,
                        });
                    } else if let Some(mut native) = self.natives.remove(&fname) {
                        let mut call_args = Vec::with_capacity(args.len() + 1);
                        call_args.push(recv);
                        call_args.extend(args);
                        let result = {
                            let mut ctx = NativeCtx {
                                heap: &mut self.heap,
                                globals: &mut self.globals,
                            };
                            native(&mut ctx, call_args)
                        };
                        self.natives.insert(fname, native);
                        self.stack.push(result?);
                    } else if let Value::Handle(h) = &recv {
                        // 无方法时：表字段读写（`sprite.x` / `sprite.x = 1`）。
                        if let Some(field) = method.strip_suffix('=') {
                            let val = args.last().cloned().unwrap_or(Value::Null);
                            if let Ok(GcObject::Table(map)) = self.heap.get_mut(*h) {
                                map.insert(field.to_string(), val.clone());
                            }
                            self.stack.push(val);
                        } else if argc == 0 {
                            let v = match self.heap.get(*h) {
                                Ok(GcObject::Table(map)) => {
                                    map.get(&method).cloned().unwrap_or(Value::Null)
                                }
                                _ => Value::Null,
                            };
                            self.stack.push(v);
                        } else {
                            self.stack.push(Value::Null);
                        }
                    } else {
                        // 缺方法：RGSS 宿主阶段返回 nil，避免整包因缺 stub 立刻崩。
                        self.stack.push(Value::Null);
                    }
                }
                Op::GetField => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let field = self.module.functions[func_idx]
                        .strings
                        .get(idx as usize)
                        .cloned()
                        .ok_or(VmError::CodeOob)?;
                    let obj = self.pop()?;
                    let Value::Handle(h) = obj else {
                        return Err(VmError::TypeError {
                            expected: "object",
                            got: obj.type_name().into(),
                        });
                    };
                    let v = match self.heap.get(h).map_err(|_| VmError::BadHandle)? {
                        GcObject::Table(map) => map.get(&field).cloned().unwrap_or(Value::Null),
                        _ => Value::Null,
                    };
                    self.stack.push(v);
                }
                Op::SetField => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let field = self.module.functions[func_idx]
                        .strings
                        .get(idx as usize)
                        .cloned()
                        .ok_or(VmError::CodeOob)?;
                    let value = self.pop()?;
                    let obj = self.pop()?;
                    let Value::Handle(h) = obj else {
                        return Err(VmError::TypeError {
                            expected: "object",
                            got: obj.type_name().into(),
                        });
                    };
                    match self.heap.get_mut(h).map_err(|_| VmError::BadHandle)? {
                        GcObject::Table(map) => {
                            map.insert(field, value.clone());
                        }
                        _ => {
                            return Err(VmError::TypeError {
                                expected: "table",
                                got: "object".into(),
                            });
                        }
                    }
                    self.stack.push(value);
                }
                Op::Dup => {
                    let v = self.peek()?.clone();
                    self.stack.push(v);
                }
                Op::NewTable => {
                    let h = self.heap.alloc(GcObject::Table(HashMap::new()));
                    self.stack.push(Value::Handle(h));
                }
                Op::NewArray => {
                    let mut ip = self.frames[fi].ip;
                    let n = Self::read_u8(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    if self.stack.len() < n as usize {
                        return Err(VmError::StackUnderflow);
                    }
                    let start = self.stack.len() - n as usize;
                    let elems: Vec<Value> = self.stack.drain(start..).collect();
                    let h = self.heap.alloc(GcObject::Array(elems));
                    self.stack.push(Value::Handle(h));
                }
            }

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
            Value::Entity(id) => format!("entity:{id}"),
            Value::Func(i) => format!("<fn {}>", i),
            Value::Handle(h) => match self.heap.get(*h) {
                Ok(GcObject::String(s)) => s.clone(),
                Ok(_) => format!("<object {}>", h.0),
                Err(_) => "<dangling>".into(),
            },
        })
    }
}

pub(crate) fn decode_op(op: u8) -> Option<Op> {
    Some(match op {
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
        x if x == Op::Mod as u8 => Op::Mod,
        x if x == Op::Send as u8 => Op::Send,
        x if x == Op::GetField as u8 => Op::GetField,
        x if x == Op::SetField as u8 => Op::SetField,
        x if x == Op::Dup as u8 => Op::Dup,
        x if x == Op::NewTable as u8 => Op::NewTable,
        x if x == Op::NewArray as u8 => Op::NewArray,
        x if x == Op::CallHost as u8 => Op::CallHost,
        _ => return None,
    })
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Entity(x), Value::Entity(y)) => x == y,
        (Value::Func(x), Value::Func(y)) => x == y,
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

    #[test]
    fn call_function_by_name() {
        let mut add = FuncProto::new("add", 2);
        add.locals = 2;
        add.emit(Op::LoadLocal);
        add.emit_u16(0);
        add.emit(Op::LoadLocal);
        add.emit_u16(1);
        add.emit(Op::Add);
        add.emit(Op::Return);
        let main = FuncProto::new("__main", 0);
        let mut vm = Vm::new(Module {
            functions: vec![add, main],
            entry: 1,
            native_names: Vec::new(),
        });
        let mut host = BufHost(String::new());
        let v = vm
            .call_function("add", &[Value::Number(40.0), Value::Number(2.0)], &mut host)
            .unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn call_host_slot_dispatches_by_prepared_name() {
        let mut f = FuncProto::new("__main", 0);
        let c = f.add_const_number(21.0);
        f.emit(Op::LoadConst);
        f.emit_u16(c);
        f.emit(Op::CallHost);
        f.emit_u16(0);
        f.emit_u8(1);
        f.emit(Op::Return);
        let mut vm = Vm::new(Module {
            functions: vec![f],
            entry: 0,
            native_names: vec!["double".into()],
        });
        vm.prepare_host_slots(["double"]);
        vm.register_native("double", |_ctx, args| {
            let n = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
            Ok(Value::Number(n * 2.0))
        });
        let mut host = BufHost(String::new());
        let v = vm.run(&mut host).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
        assert_eq!(vm.call_hits.get("host:0:double"), Some(&1));
    }
}
