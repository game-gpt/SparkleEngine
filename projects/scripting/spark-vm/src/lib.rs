//! Spark VM：栈式字节码解释器（无游戏语义）。
//!
//! # 与 ECS / JIT 的边界
//!
//! - **ECS**：世界与 Component 在 `spark-ecs`；脚本经 [`Value::Entity`] 与
//!   [`Op::CallHost`] 交换不透明 ID。调度侧用 [`Vm::call_function`]
//!   调脚本导出，勿把 World 塞进 VM。
//! - **JIT**：每帧解释累加 [`Vm::hotness`]；`spark-jit` 对热点 [`FuncProto`] 做字节码特化，
//!   [`Op::JitEnter`] 预留原生 stub 槽（解释路径跳过）。
//! - **栈式**：操作数在值栈，调用帧只记 `func` / `ip` / `stack_base`，利于特化与调试。

#![forbid(missing_docs)]
use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
};

use spark_diagnostics::{Error, ErrorArg, ErrorArgs, ErrorCode};
use spark_gc::{GcObject, Heap, Value};

mod bind;
mod verify;

pub use bind::reject_residual_call_native;
pub use verify::{BytecodeVerifyError, verify_bytecode, verify_bytecode_with_host};

/// VM 结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum VmError {
    /// 值栈弹出时为空（操作数不足）。
    StackUnderflow,
    /// 指令指针越过当前帧 `code` 边界，或操作数截断。
    CodeOob,
    /// 运行时类型不符合操作码期望（算术 / 比较 / 字段访问等）。
    TypeError {
        /// 期望的类型名（诊断用静态标签）。
        expected: &'static str,
        /// 实际值的类型描述。
        got: String,
    },
    /// 全局符号表中找不到该名字。
    UnknownGlobal(
        /// 缺失的全局名。
        String,
    ),
    /// 模块函数表中找不到该导出名。
    UnknownFunction(
        /// 缺失的函数名。
        String,
    ),
    /// 调用帧栈溢出（达到硬上限，先于预算检查的保护）。
    CallOverflow,
    /// `Return` 时无活动帧，或返回值无法落到调用方栈槽。
    BadReturn,
    /// 除法 / 取模除零（算术指令当前压 `0`；宿主 / 扩展路径可返回此码）。
    DivByZero,
    /// 未注册且无法默认占位的原生 / 宿主名。
    UnknownNative(
        /// 缺失的原生函数名。
        String,
    ),
    /// 字节流中出现无法映射到 [`Op`] 的操作码字节。
    UnknownOpcode(
        /// 原始操作码字节。
        u8,
    ),
    /// 调用实参个数与 [`FuncProto::arity`] 不一致。
    ArityMismatch {
        /// 原型声明的形参数。
        expected: u16,
        /// 实际压入的实参数。
        got: u16,
    },
    /// 原生 / 宿主闭包拒绝某个实参形态。
    BadNativeArg {
        /// 被调用的原生名（稳定诊断键）。
        name: &'static str,
    },
    /// 堆句柄无效。
    BadHandle,
    /// 指令步数预算耗尽。
    StepLimitExceeded,
    /// 宿主调用次数预算耗尽。
    HostCallLimitExceeded,
    /// 脚本调用深度预算耗尽。
    CallDepthExceeded,
    /// 堆分配次数预算耗尽。
    AllocationLimitExceeded,
    /// 宿主 ABI / 阶段 / 能力门禁拒绝。
    HostDenied {
        /// 稳定拒绝令牌（供诊断与本地化，非自由文本）。
        detail: String,
    },
}

impl VmError {
    /// 稳定错误码字符串（`spark.vm.*`），供诊断与本地化键使用。
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
            Self::StepLimitExceeded => "spark.vm.step_limit",
            Self::HostCallLimitExceeded => "spark.vm.host_call_limit",
            Self::CallDepthExceeded => "spark.vm.call_depth_limit",
            Self::AllocationLimitExceeded => "spark.vm.allocation_limit",
            Self::HostDenied { .. } => "spark.vm.host_denied",
        }
    }

    /// 类型化参数（供 Diagnostic / LogEvent / localization 渲染）。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::TypeError { expected, got } => ErrorArgs::new()
                .with("expected", ErrorArg::TypeName(Arc::from(*expected)))
                .with("got", ErrorArg::TypeName(Arc::from(got.as_str()))),
            Self::UnknownGlobal(name) | Self::UnknownFunction(name) | Self::UnknownNative(name) => {
                ErrorArgs::new().with("name", ErrorArg::String(Arc::from(name.as_str())))
            }
            Self::UnknownOpcode(op) => ErrorArgs::new().with("opcode", ErrorArg::Opcode(*op)),
            Self::ArityMismatch { expected, got } => {
                ErrorArgs::new().with("expected", ErrorArg::Unsigned(u64::from(*expected))).with("got", ErrorArg::Unsigned(u64::from(*got)))
            }
            Self::BadNativeArg { name } => ErrorArgs::new().with("name", ErrorArg::String(Arc::from(*name))),
            Self::HostDenied { detail } => ErrorArgs::new().with("detail", ErrorArg::String(Arc::from(detail.as_str()))),
            Self::StackUnderflow
            | Self::CodeOob
            | Self::CallOverflow
            | Self::BadReturn
            | Self::DivByZero
            | Self::BadHandle
            | Self::StepLimitExceeded
            | Self::HostCallLimitExceeded
            | Self::CallDepthExceeded
            | Self::AllocationLimitExceeded => ErrorArgs::new(),
        }
    }

    /// 转为 `spark-diagnostics` 的 [`Error`]（稳定码 + 类型化参数）。
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
    /// 空操作：不改值栈与指令指针以外的状态。
    Nop = 0,
    /// 压入 [`Value::Null`]。
    LoadNull,
    /// 压入 [`Value::Bool`]`(true)`。
    LoadTrue,
    /// 压入 [`Value::Bool`]`(false)`。
    LoadFalse,
    /// 后跟 u16 常量下标；从当前帧 [`FuncProto::consts`] 压入一份克隆。
    LoadConst,
    /// 后跟 u16 局部槽；相对当前帧 `stack_base` 读槽并压栈（越界读为 `Null`）。
    LoadLocal,
    /// 后跟 u16 局部槽；弹出栈顶写入该槽（不足则扩展栈垫 `Null`）。
    StoreLocal,
    /// 后跟 u16 全局名常量表下标；按 [`FuncProto::const_names`] 查 [`Vm::globals`]（未定义压 `Null`）。
    LoadGlobal,
    /// 后跟 u16 全局名常量表下标；弹出栈顶写入 [`Vm::globals`]。
    StoreGlobal,
    /// 弹出 `b` 再 `a`：数字相加；数组拼接；其余按字符串拼接后入堆。
    Add,
    /// 弹出两操作数，按数字减法（非数字按 0）。
    Sub,
    /// 弹出两操作数：数组/字符串与数字做重复；否则按数字乘法。
    Mul,
    /// 弹出两操作数，按数字除法（除零结果为 0，不抛 [`VmError::DivByZero`]）。
    Div,
    /// 弹出一操作数，压入其数字取负（非数字按 0）。
    Neg,
    /// 弹出一操作数，压入其真值取反（[`Value`] 真值语义）。
    Not,
    /// 弹出两操作数，按值相等比较，压入 [`Value::Bool`]。
    Eq,
    /// 弹出两操作数，按值不等比较，压入 [`Value::Bool`]。
    Ne,
    /// 弹出两操作数，按数字 `<` 比较（非数字按 0）。
    Lt,
    /// 弹出两操作数，按数字 `<=` 比较（非数字按 0）。
    Le,
    /// 弹出两操作数，按数字 `>` 比较（非数字按 0）。
    Gt,
    /// 弹出两操作数，按数字 `>=` 比较（非数字按 0）。
    Ge,
    /// 相对跳转：后跟 i16，目标为「读完操作数后的 ip + 偏移」。
    Jump,
    /// 条件跳转：后跟 i16；弹出条件，假值则相对跳转，真值则落到操作数之后。
    JumpIfFalse,
    /// 条件跳转：后跟 i16；弹出条件，真值则相对跳转，假值则落到操作数之后。
    JumpIfTrue,
    /// 脚本调用：后跟 u8 参数个数；栈顶为 `argN-1…arg0`，其下为 callee（[`Value::Func`]）。
    /// 缺参垫 `Null`、多余实参丢弃；受 [`Vm::call_depth_limit`] 约束。
    Call,
    /// 弹出返回值（空栈则 `Null`），弹出当前帧并截断到 `stack_base`，再把返回值压给调用方。
    Return,
    /// 弹出 N 个栈值：后跟 u8 个数；不足则 [`VmError::StackUnderflow`]。
    Pop,
    /// 打印栈顶（不弹出）：经 [`HostHooks::print`] 输出。
    Print,
    /// JIT 入口占位：后跟 u32 stub id；解释路径只跳过操作数，不进入原生 stub。
    JitEnter,
    /// 后跟 u16 字符串池下标；从 [`FuncProto::strings`] 分配堆字符串并压栈。
    LoadString,
    /// 保留操作码：正式制品禁止出现；链接 / 验证拒绝，解释期 trap。
    /// 宿主调用一律用 [`Op::CallHost`]。
    CallNative,
    /// 取模：弹出两操作数按数字取余（除零结果为 0）。
    Mod,
    /// 实例方法派发：后跟 u16 方法名字符串池下标 + u8 参数个数（不含接收者）。
    /// 栈：`recv, arg0…argN-1`。按 `recv.__class` + 方法名查找 `Class_method`。
    Send,
    /// 读表字段：后跟 u16 字符串池下标。弹出对象，压入字段值。
    GetField,
    /// 写表字段：后跟 u16 字符串池下标。弹出值再弹出对象，写入后压回值。
    SetField,
    /// 复制栈顶（再压一份相同值）。
    Dup,
    /// 新建空表：无操作数，压入 `Table` 句柄。
    NewTable,
    /// 新建数组：后跟 u8 元素个数；弹出 N 个元素（底→顶为 0..N-1），压入 `Array`。
    NewArray,
    /// 宿主槽位调用：后跟 u16 槽位 + u8 参数个数（链接后 ABI；不经字符串查找）。
    /// 受 [`Vm::host_call_limit`] 约束。
    CallHost,
}

/// 编译期函数原型（解释与 JIT 共用）。
#[derive(Debug, Clone)]
pub struct FuncProto {
    /// 导出名（链接按名合并；[`Module::find_function`] / [`Op::Send`] 派发键）。
    pub name: String,
    /// 形参数（与 [`Op::Call`] / [`Vm::call_function`] 实参个数对齐）。
    pub arity: u8,
    /// 局部槽总数（含参数）；帧进入时值栈从 `stack_base` 至少扩展到此长度。
    pub locals: u16,
    /// 紧凑字节码：操作码字节 + 小端操作数。
    pub code: Vec<u8>,
    /// 常量表：[`Op::LoadConst`] 按下标克隆压栈；可含 [`Value::Func`] 等。
    pub consts: Vec<Value>,
    /// 与 `consts` 并行的名字槽（全局符号用）。
    pub const_names: Vec<String>,
    /// 字符串池：[`Op::LoadString`] / [`Op::Send`] / 字段名等按下标引用。
    pub strings: Vec<String>,
}

impl FuncProto {
    /// 新建空原型：`locals` 初值等于 `arity`，常量表与码流为空。
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

    /// 追加一个无操作数的操作码字节。
    pub fn emit(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    /// 追加一个 u8 操作数（如 `Call`/`Pop`/`NewArray` 的个数）。
    pub fn emit_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    /// 追加一个小端 u16 操作数（常量/局部/字符串下标等）。
    pub fn emit_u16(&mut self, v: u16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    /// 追加一个小端 i16 操作数（相对跳转偏移）。
    pub fn emit_i16(&mut self, v: i16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    /// 向常量表追加数字，返回可供 [`Op::LoadConst`] 使用的下标。
    pub fn add_const_number(&mut self, n: f64) -> u16 {
        let i = self.consts.len() as u16;
        self.consts.push(Value::Number(n));
        self.const_names.push(String::new());
        i
    }

    /// 向常量表追加函数下标（[`Value::Func`]），返回 `LoadConst` 下标。
    pub fn add_const_func(&mut self, func: u32) -> u16 {
        let i = self.consts.len() as u16;
        self.consts.push(Value::Func(func));
        self.const_names.push(String::new());
        i
    }

    /// 登记全局名槽：`consts` 占位 `Null`，`const_names` 存名字；供 [`Op::LoadGlobal`]/[`Op::StoreGlobal`]。
    pub fn add_const_name(&mut self, name: impl Into<String>) -> u16 {
        let name = name.into();
        let i = self.consts.len() as u16;
        self.consts.push(Value::Null);
        self.const_names.push(name);
        i
    }

    /// 向字符串池追加字面量，返回 [`Op::LoadString`] / 方法名等可用的下标。
    pub fn add_string(&mut self, s: impl Into<String>) -> u16 {
        let i = self.strings.len() as u16;
        self.strings.push(s.into());
        i
    }

    /// 回填已预留的小端 i16（典型：先 `emit(Jump*)` 再 `emit_i16(0)`，末尾再 `patch_i16`）。
    pub fn patch_i16(&mut self, at: usize, v: i16) {
        let b = v.to_le_bytes();
        self.code[at] = b[0];
        self.code[at + 1] = b[1];
    }

    /// 当前 `code` 字节长度（下一写位置 / 跳转锚点）。
    pub fn len(&self) -> usize {
        self.code.len()
    }
}

/// 已链接的脚本模块：函数表 + 入口 +（遗留）原生名表。
#[derive(Debug, Clone)]
pub struct Module {
    /// 函数原型表；[`Value::Func`] 与调用下标均相对此表。
    pub functions: Vec<FuncProto>,
    /// 入口函数下标（块初始化 / REPL；模组语义入口用命名导出）。
    pub entry: usize,
    /// 遗留原生名表（链接合并用；正式宿主调用走 [`Op::CallHost`] 槽位）。
    pub native_names: Vec<String>,
}

impl Module {
    /// 以给定函数表与入口下标构造模块（`native_names` 为空）。
    pub fn with_entry(functions: Vec<FuncProto>, entry: usize) -> Self {
        Self { functions, entry, native_names: Vec::new() }
    }

    /// 按导出名查找函数下标（首次匹配）；供 [`Vm::call_function`] / [`Op::Send`]。
    pub fn find_function(&self, name: &str) -> Option<usize> {
        self.functions.iter().position(|f| f.name == name)
    }

    /// 将原生名登记进 `native_names`（已存在则返回原下标）。
    pub fn intern_native(&mut self, name: impl Into<String>) -> u16 {
        let name = name.into();
        if let Some(i) = self.native_names.iter().position(|n| n == &name) {
            return i as u16;
        }
        let i = self.native_names.len() as u16;
        self.native_names.push(name);
        i
    }

    /// 把多个脚本模块的方法链进同一命名空间（同名**先到先得**，依赖包应排在入口之前）。
    ///
    /// 不合成假入口；若无函数则插入空 `on_load` stub。
    pub fn link_methods(modules: &[Module]) -> Module {
        let mut name_to_idx: HashMap<String, usize> = HashMap::new();
        let mut functions: Vec<FuncProto> = Vec::new();
        let mut native_names: Vec<String> = Vec::new();
        let mut filled: HashSet<String> = HashSet::new();

        for module in modules {
            for name in &module.native_names {
                if !native_names.iter().any(|n| n == name) {
                    native_names.push(name.clone());
                }
            }
            for func in &module.functions {
                if !name_to_idx.contains_key(&func.name) {
                    name_to_idx.insert(func.name.clone(), functions.len());
                    functions.push(FuncProto::new(func.name.clone(), func.arity));
                }
            }
        }

        let native_to_idx: HashMap<String, u16> = native_names.iter().enumerate().map(|(i, n)| (n.clone(), i as u16)).collect();

        for module in modules {
            for func in &module.functions {
                if filled.contains(&func.name) {
                    continue;
                }
                filled.insert(func.name.clone());
                let idx = name_to_idx[&func.name];
                functions[idx] = remap_func_against(module, func, &name_to_idx, &native_to_idx);
            }
        }

        if functions.is_empty() {
            let mut stub = FuncProto::new("on_load", 0);
            stub.emit(Op::LoadNull);
            stub.emit(Op::Return);
            functions.push(stub);
        }

        Module { functions, entry: 0, native_names }
    }

    /// 合并多个模块，并把 `modules[entry_index]` 的入口函数设为链接结果的 `entry`。
    ///
    /// 保留入口函数原名（通常为 `on_load`）。
    pub fn link_with_entry(modules: &[Module], entry_index: usize) -> Result<Module, String> {
        if modules.is_empty() {
            return Err("empty_link_set".into());
        }
        if entry_index >= modules.len() {
            return Err(format!("entry_oob:{entry_index}"));
        }
        let mut linked = Self::link_methods(modules);
        let entry_mod = &modules[entry_index];
        let main = entry_mod.functions.get(entry_mod.entry).ok_or_else(|| "missing_entry_func".to_string())?;
        let mut name_to_idx: HashMap<String, usize> = linked.functions.iter().enumerate().map(|(i, f)| (f.name.clone(), i)).collect();
        let native_to_idx: HashMap<String, u16> = linked.native_names.iter().enumerate().map(|(i, n)| (n.clone(), i as u16)).collect();
        let remapped = remap_func_against(entry_mod, main, &name_to_idx, &native_to_idx);
        let entry_name = remapped.name.clone();
        if let Some(&idx) = name_to_idx.get(&entry_name) {
            linked.functions[idx] = remapped;
            linked.entry = idx;
        }
        else {
            linked.entry = linked.functions.len();
            name_to_idx.insert(entry_name, linked.entry);
            linked.functions.push(remapped);
        }
        Ok(linked)
    }
}

/// 按「旧模块下标 → 函数名 → 新模块下标」重写 `Value::Func`。
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
    /// 可变堆：分配字符串 / 表 / 数组，解析 [`Value::Handle`]。
    pub heap: &'a mut Heap,
    /// 可变全局符号表（与 [`Op::LoadGlobal`] / [`Op::StoreGlobal`] 同一映射）。
    pub globals: &'a mut HashMap<String, Value>,
}

/// 已装箱的原生 / 宿主闭包：接收上下文与实参向量，返回值或 [`VmError`]。
pub type NativeFn = Box<dyn FnMut(&mut NativeCtx<'_>, Vec<Value>) -> Result<Value, VmError>>;

#[derive(Debug)]
struct Frame {
    /// 当前帧执行的 [`Module::functions`] 下标。
    func: usize,
    /// 下一待取指令在该函数 `code` 中的字节偏移。
    ip: usize,
    /// 该帧在值栈上的基址（含参数）。
    stack_base: usize,
}

/// 宿主钩子（打印等；ECS 侧可换实现）。
pub trait HostHooks {
    /// 输出一行调试 / `Print` 操作码文本（不含末尾换行约定由实现决定）。
    fn print(&mut self, text: &str);
}

/// 默认宿主：[`HostHooks::print`] 转发到标准输出。
pub struct StdHost;

impl HostHooks for StdHost {
    fn print(&mut self, text: &str) {
        println!("{text}");
    }
}

/// 栈式字节码虚拟机：持有模块、堆、全局、调用帧与解释预算。
pub struct Vm {
    /// 当前装载的脚本模块（函数表与入口）。
    pub module: Module,
    /// 对象堆（字符串 / 表 / 数组）；GC 根来自值栈与全局。
    pub heap: Heap,
    /// 全局符号表（[`Op::LoadGlobal`] / [`Op::StoreGlobal`]）。
    pub globals: HashMap<String, Value>,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    /// 每条函数解释步热度（供 JIT）。
    pub hotness: Vec<u32>,
    /// 按限定名注册的原生 / 宿主实现（[`Op::CallHost`] / [`Op::Send`] 回退）。
    pub natives: HashMap<String, NativeFn>,
    /// 宿主槽位 → 调度名（与 [`HostFunctionId::qualified_name`] / `register_native` 键一致，顺序 = 槽位）。
    pub host_slot_names: Vec<String>,
    /// 单次 `interpret` 步数上限。
    pub step_limit: u64,
    /// 单次 `interpret` 宿主调用（`CallHost` / `Send` 内建以外）上限。
    pub host_call_limit: u64,
    /// 脚本调用帧深度上限。
    pub call_depth_limit: u16,
    /// 单次 `interpret` 期间允许的堆分配次数上限（相对入口时的 `heap.total_allocs`）。
    pub allocation_limit: u64,
    /// 当前 `interpret` 已发生的宿主调用次数。
    host_calls: u64,
    /// 当前 `interpret` 入口时的堆分配计数快照。
    allocs_at_entry: u64,
    /// CallHost / Send 调用计数（诊断用）。
    pub call_hits: HashMap<String, u32>,
}

impl Vm {
    /// 以模块构造 VM：默认预算（步数 / 宿主调用 / 深度 / 分配）已设，热度表与函数表等长。
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
            host_call_limit: 100_000,
            call_depth_limit: 256,
            allocation_limit: 1_000_000,
            host_calls: 0,
            allocs_at_entry: 0,
            call_hits: HashMap::new(),
        }
    }

    /// 装载编译期宿主槽位表（调度名顺序 = [`HostSchema`] / `HostId` 槽位下标）。
    pub fn prepare_host_slots(&mut self, names: impl IntoIterator<Item = impl Into<String>>) {
        self.host_slot_names = names.into_iter().map(Into::into).collect();
    }

    /// 按名字注册原生闭包（覆盖同名）；[`Op::CallHost`] 经槽位解析到此键。
    pub fn register_native<F>(&mut self, name: impl Into<String>, f: F)
    where
        F: FnMut(&mut NativeCtx<'_>, Vec<Value>) -> Result<Value, VmError> + 'static,
    {
        self.natives.insert(name.into(), Box::new(f));
    }

    /// 注册已装箱的原生闭包（插件重映射限定名用）。
    pub fn register_native_fn(&mut self, name: impl Into<String>, f: NativeFn) {
        self.natives.insert(name.into(), f);
    }

    /// 取出已注册原生（不存在则 `None`）。
    pub fn take_native(&mut self, name: &str) -> Option<NativeFn> {
        self.natives.remove(name)
    }

    /// 从模块入口运行（块初始化 / REPL；模组请用命名导出）。
    pub fn run(&mut self, host: &mut dyn HostHooks) -> Result<Value, VmError> {
        let entry = self.module.entry;
        self.call_index(entry, &[], host)
    }

    /// 在已链接方法表上执行某一脚本的入口函数（按 `script.entry` 重写下标后调用）。
    pub fn run_script_main(&mut self, script: &Module, host: &mut dyn HostHooks) -> Result<Value, VmError> {
        let main = script.functions.get(script.entry).ok_or(VmError::CodeOob)?;
        let name_to_idx: HashMap<String, usize> = self.module.functions.iter().enumerate().map(|(i, f)| (f.name.clone(), i)).collect();
        let native_to_idx: HashMap<String, u16> = self.module.native_names.iter().enumerate().map(|(i, n)| (n.clone(), i as u16)).collect();
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
    pub fn call_function(&mut self, name: &str, args: &[Value], host: &mut dyn HostHooks) -> Result<Value, VmError> {
        let idx = self.module.find_function(name).ok_or_else(|| VmError::UnknownFunction(name.into()))?;
        self.call_index(idx, args, host)
    }

    fn invoke_native(&mut self, name: &str, args: Vec<Value>) -> Result<Value, VmError> {
        self.host_calls += 1;
        if self.host_calls > self.host_call_limit {
            return Err(VmError::HostCallLimitExceeded);
        }
        if !self.natives.contains_key(name) {
            // 未注册 native 默认空实现，便于逐步补齐 RGSS API。
            self.natives.insert(name.to_string(), Box::new(|_ctx, _args| Ok(Value::Null)));
        }
        let mut native = self.natives.remove(name).ok_or_else(|| VmError::UnknownNative(name.to_string()))?;
        let result = {
            let mut ctx = NativeCtx { heap: &mut self.heap, globals: &mut self.globals };
            native(&mut ctx, args)
        };
        self.natives.insert(name.to_string(), native);
        result
    }

    fn call_index(&mut self, func: usize, args: &[Value], host: &mut dyn HostHooks) -> Result<Value, VmError> {
        let arity = self.module.functions[func].arity as usize;
        if args.len() != arity {
            return Err(VmError::ArityMismatch { expected: arity as u16, got: args.len() as u16 });
        }
        self.frames.clear();
        self.stack.clear();
        self.host_calls = 0;
        self.allocs_at_entry = self.heap.total_allocs;
        self.stack.extend_from_slice(args);
        let need = self.module.functions[func].locals as usize;
        while self.stack.len() < need {
            self.stack.push(Value::Null);
        }
        self.frames.push(Frame { func, ip: 0, stack_base: 0 });
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

    fn bin_num(&mut self, op: impl Fn(f64, f64) -> Result<f64, VmError>) -> Result<(), VmError> {
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
                        best_name = self.module.functions.get(i).map(|f| f.name.clone()).unwrap_or_else(|| "?".into());
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
                return Err(VmError::StepLimitExceeded);
            }
            if self.heap.total_allocs.saturating_sub(self.allocs_at_entry) > self.allocation_limit {
                return Err(VmError::AllocationLimitExceeded);
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
            let op = decode_op(op_byte).ok_or_else(|| VmError::UnknownOpcode(op_byte))?;

            match op {
                Op::Nop => {}
                Op::LoadNull => self.stack.push(Value::Null),
                Op::LoadTrue => self.stack.push(Value::Bool(true)),
                Op::LoadFalse => self.stack.push(Value::Bool(false)),
                Op::LoadConst => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let c = self.module.functions[func_idx].consts.get(idx as usize).cloned().ok_or(VmError::CodeOob)?;
                    self.stack.push(c);
                }
                Op::LoadLocal => {
                    let mut ip = self.frames[fi].ip;
                    let slot = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let base = self.frames[fi].stack_base;
                    let v = self.stack.get(base + slot as usize).cloned().unwrap_or(Value::Null);
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
                    let name = self.module.functions[func_idx].const_names.get(idx as usize).cloned().unwrap_or_default();
                    // Ruby 未定义全局读为 nil；常量未定义暂同此处理，便于 stub 宿主。
                    let v = self.globals.get(&name).cloned().unwrap_or(Value::Null);
                    self.stack.push(v);
                }
                Op::StoreGlobal => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let name = self.module.functions[func_idx].const_names.get(idx as usize).cloned().unwrap_or_default();
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
                        (Value::Handle(ha), Value::Handle(hb)) => match (self.heap.get(*ha), self.heap.get(*hb)) {
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
                        },
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
                        (Value::Handle(h), Value::Number(n)) => match self.heap.get(*h) {
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
                        },
                        _ => {
                            let an = a.as_number().unwrap_or(0.0);
                            let bn = b.as_number().unwrap_or(0.0);
                            self.stack.push(Value::Number(an * bn));
                        }
                    }
                }
                Op::Sub => self.bin_num(|a, b| Ok(a - b))?,
                Op::Div => self.bin_num(|a, b| if b == 0.0 { Ok(0.0) } else { Ok(a / b) })?,
                Op::Mod => self.bin_num(|a, b| if b == 0.0 { Ok(0.0) } else { Ok(a % b) })?,
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
                    }
                    else {
                        self.frames[fi].ip = ip;
                    }
                }
                Op::JumpIfTrue => {
                    let mut ip = self.frames[fi].ip;
                    let off = Self::read_i16(&self.module.functions[func_idx].code, &mut ip)?;
                    let cond = self.pop()?;
                    if cond.truthy() {
                        self.frames[fi].ip = ((ip as isize) + off as isize) as usize;
                    }
                    else {
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
                    let fidx =
                        callee.as_func().ok_or_else(|| VmError::TypeError { expected: "function", got: callee.type_name().into() })? as usize;
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
                        }
                        else {
                            let drop_n = (argc - arity) as usize;
                            let start = self.stack.len() - drop_n;
                            self.stack.truncate(start);
                            argc = arity;
                        }
                    }
                    if self.frames.len() as u16 >= self.call_depth_limit {
                        let stack: Vec<_> = self
                            .frames
                            .iter()
                            .rev()
                            .take(10)
                            .map(|f| self.module.functions.get(f.func).map(|p| p.name.as_str()).unwrap_or("?"))
                            .collect();
                        eprintln!("rgss play: call depth overflow top={}", stack.join(" <- "));
                        return Err(VmError::CallDepthExceeded);
                    }
                    self.stack.remove(callee_idx);
                    let base = self.stack.len() - arity as usize;
                    let need = self.module.functions[fidx].locals as usize;
                    while self.stack.len() < base + need {
                        self.stack.push(Value::Null);
                    }
                    self.frames.push(Frame { func: fidx, ip: 0, stack_base: base });
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
                    let s = self.module.functions[func_idx].strings.get(idx as usize).cloned().ok_or(VmError::CodeOob)?;
                    let v = self.heap.alloc_string(s);
                    self.stack.push(v);
                }
                Op::CallNative => {
                    // 正式路径只发射 CallHost；残留 CallNative 一律 trap。
                    return Err(VmError::UnknownOpcode(Op::CallNative as u8));
                }
                Op::CallHost => {
                    let mut ip = self.frames[fi].ip;
                    let slot = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    let argc = Self::read_u8(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let name =
                        self.host_slot_names.get(slot as usize).cloned().ok_or_else(|| VmError::UnknownNative(format!("host_slot:{slot}")))?;
                    if self.stack.len() < argc as usize {
                        return Err(VmError::StackUnderflow);
                    }
                    let args: Vec<Value> = self.stack.drain(self.stack.len() - argc as usize..).collect();
                    *self.call_hits.entry(format!("host:{slot}:{name}")).or_insert(0) += 1;
                    let result = self.invoke_native(&name, args)?;
                    self.stack.push(result);
                }
                Op::Send => {
                    let mut ip = self.frames[fi].ip;
                    let method_idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    let argc = Self::read_u8(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let method = self.module.functions[func_idx].strings.get(method_idx as usize).cloned().ok_or(VmError::CodeOob)?;
                    if self.stack.len() < argc as usize + 1 {
                        return Err(VmError::StackUnderflow);
                    }
                    let args: Vec<Value> = self.stack.drain(self.stack.len() - argc as usize..).collect();
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
                                    let i = if idx < 0 { (arr.len() as isize + idx) as usize } else { idx as usize };
                                    let v = arr.get(i).cloned().unwrap_or(Value::Null);
                                    self.stack.push(v);
                                    continue;
                                }
                                "[]" if argc == 2 => {
                                    // `arr[start, length]` 切片。
                                    let start = args.first().and_then(|v| v.as_number()).unwrap_or(0.0) as isize;
                                    let len = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0).max(0.0) as usize;
                                    let start = if start < 0 { (arr.len() as isize + start).max(0) as usize } else { start as usize };
                                    let end = (start + len).min(arr.len());
                                    let slice = if start < arr.len() { arr[start..end].to_vec() } else { Vec::new() };
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
                        }
                        else if call_args.len() > arity_usize {
                            call_args.truncate(arity_usize);
                        }
                        if self.frames.len() as u16 >= self.call_depth_limit {
                            let stack: Vec<_> = self
                                .frames
                                .iter()
                                .rev()
                                .take(10)
                                .map(|f| self.module.functions.get(f.func).map(|p| p.name.as_str()).unwrap_or("?"))
                                .collect();
                            eprintln!("rgss play: send depth overflow top={}", stack.join(" <- "));
                            return Err(VmError::CallDepthExceeded);
                        }
                        let base = self.stack.len();
                        self.stack.extend(call_args);
                        let need = self.module.functions[fidx].locals as usize;
                        while self.stack.len() < base + need {
                            self.stack.push(Value::Null);
                        }
                        self.frames.push(Frame { func: fidx, ip: 0, stack_base: base });
                    }
                    else if let Some(mut native) = self.natives.remove(&fname) {
                        let mut call_args = Vec::with_capacity(args.len() + 1);
                        call_args.push(recv);
                        call_args.extend(args);
                        let result = {
                            let mut ctx = NativeCtx { heap: &mut self.heap, globals: &mut self.globals };
                            native(&mut ctx, call_args)
                        };
                        self.natives.insert(fname, native);
                        self.stack.push(result?);
                    }
                    else if let Value::Handle(h) = &recv {
                        // 无方法时：表字段读写（`sprite.x` / `sprite.x = 1`）。
                        if let Some(field) = method.strip_suffix('=') {
                            let val = args.last().cloned().unwrap_or(Value::Null);
                            if let Ok(GcObject::Table(map)) = self.heap.get_mut(*h) {
                                map.insert(field.to_string(), val.clone());
                            }
                            self.stack.push(val);
                        }
                        else if argc == 0 {
                            let v = match self.heap.get(*h) {
                                Ok(GcObject::Table(map)) => map.get(&method).cloned().unwrap_or(Value::Null),
                                _ => Value::Null,
                            };
                            self.stack.push(v);
                        }
                        else {
                            self.stack.push(Value::Null);
                        }
                    }
                    else {
                        // 缺方法：RGSS 宿主阶段返回 nil，避免整包因缺 stub 立刻崩。
                        self.stack.push(Value::Null);
                    }
                }
                Op::GetField => {
                    let mut ip = self.frames[fi].ip;
                    let idx = Self::read_u16(&self.module.functions[func_idx].code, &mut ip)?;
                    self.frames[fi].ip = ip;
                    let field = self.module.functions[func_idx].strings.get(idx as usize).cloned().ok_or(VmError::CodeOob)?;
                    let obj = self.pop()?;
                    let Value::Handle(h) = obj
                    else {
                        return Err(VmError::TypeError { expected: "object", got: obj.type_name().into() });
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
                    let field = self.module.functions[func_idx].strings.get(idx as usize).cloned().ok_or(VmError::CodeOob)?;
                    let value = self.pop()?;
                    let obj = self.pop()?;
                    let Value::Handle(h) = obj
                    else {
                        return Err(VmError::TypeError { expected: "object", got: obj.type_name().into() });
                    };
                    match self.heap.get_mut(h).map_err(|_| VmError::BadHandle)? {
                        GcObject::Table(map) => {
                            map.insert(field, value.clone());
                        }
                        _ => {
                            return Err(VmError::TypeError { expected: "table", got: "object".into() });
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

    /// 将 [`Value`] 格式化为可读字符串（`Print`、字符串拼接与诊断共用）。
    ///
    /// 句柄解析堆对象；悬空句柄显示为 `<dangling>`，不视为 [`VmError::BadHandle`]。
    pub fn value_to_string(&self, v: &Value) -> Result<String, VmError> {
        Ok(match v {
            Value::Null => "null".into(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if *n == n.trunc() && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                }
                else {
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
