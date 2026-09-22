//! Spark JIT：热函数检测 + 字节码特化（基线；日后可接原生后端）。
//!
//! 当前不做机器码发射：对热点 `FuncProto` 做常量折叠与死跳消除，
//! 写回模块，降低解释器派发开销。API 预留 stub 表供原生入口。

#![deny(missing_docs)]
use std::collections::HashMap;

use std::fmt;

use spark_vm::{FuncProto, Module, Op, Vm};

/// JIT 结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum JitError {
    /// 函数下标非法或无法特化。
    BadFunc,
    /// 特化扫描时操作数被截断。
    TruncatedOperands,
}

impl JitError {
    /// 稳定错误码（`spark.jit.*`）。
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadFunc => "spark.jit.bad_func",
            Self::TruncatedOperands => "spark.jit.truncated_operands",
        }
    }
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for JitError {}

/// 已特化的函数记录。
#[derive(Debug, Clone)]
pub struct JitStub {
    /// 模块内函数下标。
    pub func: usize,
    /// 触发特化时的热度计数快照。
    pub specialized_from_hotness: u32,
}

/// 热点检测与字节码特化引擎（非机器码后端）。
pub struct JitEngine {
    /// 热度达到该阈值才特化（`Vm::hotness` 计数）。
    pub threshold: u32,
    stubs: HashMap<usize, JitStub>,
}

impl Default for JitEngine {
    fn default() -> Self {
        Self::new(512)
    }
}

impl JitEngine {
    /// 以给定热度阈值构造；阈值越大，特化越晚。
    pub fn new(threshold: u32) -> Self {
        Self { threshold, stubs: HashMap::new() }
    }

    /// 已特化函数表（下标 → stub）。
    pub fn stubs(&self) -> &HashMap<usize, JitStub> {
        &self.stubs
    }

    /// 扫描 VM 热度，对超阈值函数做字节码特化。
    pub fn optimize_hot(&mut self, vm: &mut Vm) -> Result<usize, JitError> {
        let mut n = 0;
        let hot: Vec<(usize, u32)> =
            vm.hotness.iter().enumerate().filter(|(i, h)| **h >= self.threshold && !self.stubs.contains_key(i)).map(|(i, h)| (i, *h)).collect();
        for (idx, hotness) in hot {
            specialize_func(&mut vm.module.functions[idx])?;
            self.stubs.insert(idx, JitStub { func: idx, specialized_from_hotness: hotness });
            n += 1;
        }
        Ok(n)
    }
}

/// 对单个函数做安全特化：`LoadConst A; LoadConst B; {Add,Sub,Mul,Div,Mod,比较}` → 单次常量/布尔加载。
pub fn specialize_func(f: &mut FuncProto) -> Result<(), JitError> {
    let code = f.code.clone();
    let mut out = Vec::with_capacity(code.len());
    let mut i = 0;
    while i < code.len() {
        if try_fold_const_binop(f, &code, &mut i, &mut out)? {
            continue;
        }
        out.push(code[i]);
        i += 1;
        let op = *out.last().unwrap();
        let extra = operand_bytes(op);
        for _ in 0..extra {
            if i >= code.len() {
                return Err(JitError::TruncatedOperands);
            }
            out.push(code[i]);
            i += 1;
        }
    }
    f.code = out;
    Ok(())
}

fn operand_bytes(op: u8) -> usize {
    if op == Op::LoadConst as u8
        || op == Op::LoadLocal as u8
        || op == Op::StoreLocal as u8
        || op == Op::LoadGlobal as u8
        || op == Op::StoreGlobal as u8
        || op == Op::Jump as u8
        || op == Op::JumpIfFalse as u8
        || op == Op::JumpIfTrue as u8
    {
        2
    }
    else if op == Op::Call as u8 || op == Op::Pop as u8 {
        1
    }
    else if op == Op::JitEnter as u8 {
        4
    }
    else if op == Op::LoadString as u8 {
        2
    }
    else if op == Op::CallNative as u8 || op == Op::CallHost as u8 {
        3
    }
    else {
        0
    }
}

fn try_fold_const_binop(f: &mut FuncProto, code: &[u8], i: &mut usize, out: &mut Vec<u8>) -> Result<bool, JitError> {
    if *i + 6 >= code.len() {
        return Ok(false);
    }
    if code[*i] != Op::LoadConst as u8 || code[*i + 3] != Op::LoadConst as u8 {
        return Ok(false);
    }
    let bin = code[*i + 6];
    let a = u16::from_le_bytes([code[*i + 1], code[*i + 2]]) as usize;
    let b = u16::from_le_bytes([code[*i + 4], code[*i + 5]]) as usize;
    let Some(x) = f.consts.get(a).and_then(|v| v.as_number())
    else {
        return Ok(false);
    };
    let Some(y) = f.consts.get(b).and_then(|v| v.as_number())
    else {
        return Ok(false);
    };

    if bin == Op::Add as u8 || bin == Op::Sub as u8 || bin == Op::Mul as u8 || bin == Op::Div as u8 || bin == Op::Mod as u8 {
        let r = if bin == Op::Add as u8 {
            x + y
        }
        else if bin == Op::Sub as u8 {
            x - y
        }
        else if bin == Op::Mul as u8 {
            x * y
        }
        else if bin == Op::Mod as u8 {
            if y == 0.0 { 0.0 } else { x % y }
        }
        else if y == 0.0 {
            0.0
        }
        else {
            x / y
        };
        let idx = f.add_const_number(r);
        out.push(Op::LoadConst as u8);
        out.extend_from_slice(&idx.to_le_bytes());
        *i += 7;
        return Ok(true);
    }

    let cmp = if bin == Op::Eq as u8 {
        Some(x == y)
    }
    else if bin == Op::Ne as u8 {
        Some(x != y)
    }
    else if bin == Op::Lt as u8 {
        Some(x < y)
    }
    else if bin == Op::Le as u8 {
        Some(x <= y)
    }
    else if bin == Op::Gt as u8 {
        Some(x > y)
    }
    else if bin == Op::Ge as u8 {
        Some(x >= y)
    }
    else {
        None
    };
    let Some(flag) = cmp
    else {
        return Ok(false);
    };
    out.push(if flag { Op::LoadTrue as u8 } else { Op::LoadFalse as u8 });
    *i += 7;
    Ok(true)
}

/// 对整模块做一次离线特化（测试 / 启动预热）。
pub fn specialize_module(module: &mut Module) -> Result<(), JitError> {
    for f in &mut module.functions {
        specialize_func(f)?;
    }
    Ok(())
}
