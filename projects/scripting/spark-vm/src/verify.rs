//! 字节码结构验证（装载前）。
//!
//! 检查操作码、指令边界、跳转落点与常量/局部/字符串下标，不解释执行。

use crate::{FuncProto, Module, Op, decode_op};

/// 字节码验证错误。
///
/// 字段约定：`func` 为模块内函数下标，`offset` 为该函数 `code` 字节偏移（操作码处）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeVerifyError {
    /// 模块没有任何函数原型。
    EmptyModule,
    /// 入口下标越出 `functions` 长度。
    EntryOutOfBounds {
        /// 请求的入口下标。
        entry: usize,
        /// 当前函数表长度。
        len: usize,
    },
    /// 某函数的 `code` 为空。
    EmptyFunction {
        /// 空函数在模块中的下标。
        index: usize,
    },
    /// 遇到无法解码的操作码字节。
    UnknownOpcode {
        /// 所在函数下标。
        func: usize,
        /// 出错字节偏移。
        offset: usize,
        /// 原始操作码字节。
        op: u8,
    },
    /// 操作数被截断（指令末尾不够读完操作数）。
    TruncatedOperand {
        /// 所在函数下标。
        func: usize,
        /// 该指令操作码偏移。
        offset: usize,
    },
    /// 跳转目标落在某条指令中间，而非指令边界。
    JumpOffBoundary {
        /// 所在函数下标。
        func: usize,
        /// 跳转指令偏移。
        offset: usize,
        /// 计算出的目标字节偏移。
        target: usize,
    },
    /// 跳转目标为负或越过 `code` 末尾（末尾本身允许，表示落到函数外）。
    JumpOutOfBounds {
        /// 所在函数下标。
        func: usize,
        /// 跳转指令偏移。
        offset: usize,
        /// 带符号的目标偏移（可为负）。
        target: isize,
    },
    /// 局部槽下标 ≥ `FuncProto::locals`。
    LocalOob {
        /// 所在函数下标。
        func: usize,
        /// 指令偏移。
        offset: usize,
        /// 请求的局部槽。
        slot: u16,
        /// 该函数声明的局部槽数。
        locals: u16,
    },
    /// 常量或全局名常量表下标越界。
    ConstOob {
        /// 所在函数下标。
        func: usize,
        /// 指令偏移。
        offset: usize,
        /// 请求的常量表下标。
        index: u16,
        /// 相关表长度（`consts` 或 `const_names`）。
        len: usize,
    },
    /// 字符串池下标越界（亦可能用于残留 `CallNative` 名表检查）。
    StringOob {
        /// 所在函数下标。
        func: usize,
        /// 指令偏移。
        offset: usize,
        /// 请求的字符串下标。
        index: u16,
        /// 字符串池（或与 native 名表取 max）长度。
        len: usize,
    },
    /// 常量中的 [`Value::Func`] 下标越出模块函数表。
    FuncOob {
        /// 所在函数下标。
        func: usize,
        /// 指令偏移。
        offset: usize,
        /// 常量里的函数下标。
        index: u32,
        /// 模块函数个数。
        len: usize,
    },
    /// [`Op::CallHost`] 槽位 ≥ 宿主槽位数（仅 `verify_bytecode_with_host`）。
    HostSlotOob {
        /// 所在函数下标。
        func: usize,
        /// 指令偏移。
        offset: usize,
        /// 请求的宿主槽位。
        slot: u16,
        /// 允许的槽位个数。
        len: u32,
    },
    /// 封存映像中仍残留 [`Op::CallNative`]（正式路径须已改写为 [`Op::CallHost`]）。
    ResidualCallNative {
        /// 所在函数下标。
        func: usize,
        /// `CallNative` 指令偏移。
        offset: usize,
    },
    /// 函数体全程未见 [`Op::Return`]（结构化要求：至少一条返回）。
    MissingReturn {
        /// 缺少 `Return` 的函数下标。
        func: usize,
    },
}

impl BytecodeVerifyError {
    /// 稳定错误码（`spark.vm.verify.*`），供诊断与本地化键使用。
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyModule => "spark.vm.verify.empty_module",
            Self::EntryOutOfBounds { .. } => "spark.vm.verify.entry_oob",
            Self::EmptyFunction { .. } => "spark.vm.verify.empty_function",
            Self::UnknownOpcode { .. } => "spark.vm.verify.unknown_opcode",
            Self::TruncatedOperand { .. } => "spark.vm.verify.truncated_operand",
            Self::JumpOffBoundary { .. } => "spark.vm.verify.jump_off_boundary",
            Self::JumpOutOfBounds { .. } => "spark.vm.verify.jump_oob",
            Self::LocalOob { .. } => "spark.vm.verify.local_oob",
            Self::ConstOob { .. } => "spark.vm.verify.const_oob",
            Self::StringOob { .. } => "spark.vm.verify.string_oob",
            Self::FuncOob { .. } => "spark.vm.verify.func_oob",
            Self::HostSlotOob { .. } => "spark.vm.verify.host_slot_oob",
            Self::ResidualCallNative { .. } => "spark.vm.verify.residual_call_native",
            Self::MissingReturn { .. } => "spark.vm.verify.missing_return",
        }
    }
}

impl std::fmt::Display for BytecodeVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for BytecodeVerifyError {}

/// 验证模块可被安全解释（结构层面，不校验宿主槽位）。
pub fn verify_bytecode(module: &Module) -> Result<(), BytecodeVerifyError> {
    verify_bytecode_inner(module, None)
}

/// 封存映像用：结构验证 + 禁止残留 `CallNative` + `CallHost` 槽位越界检查。
pub fn verify_bytecode_with_host(module: &Module, host_slot_count: u32) -> Result<(), BytecodeVerifyError> {
    verify_bytecode_inner(module, Some(host_slot_count))
}

fn verify_bytecode_inner(module: &Module, host_slot_count: Option<u32>) -> Result<(), BytecodeVerifyError> {
    if module.functions.is_empty() {
        return Err(BytecodeVerifyError::EmptyModule);
    }
    if module.entry >= module.functions.len() {
        return Err(BytecodeVerifyError::EntryOutOfBounds { entry: module.entry, len: module.functions.len() });
    }
    let func_count = module.functions.len() as u32;
    let native_len = module.native_names.len();
    for (index, func) in module.functions.iter().enumerate() {
        verify_function(index, func, func_count, native_len, host_slot_count)?;
    }
    Ok(())
}

fn verify_function(
    func_index: usize,
    func: &FuncProto,
    func_count: u32,
    native_len: usize,
    host_slot_count: Option<u32>,
) -> Result<(), BytecodeVerifyError> {
    if func.code.is_empty() {
        return Err(BytecodeVerifyError::EmptyFunction { index: func_index });
    }

    let mut starts = Vec::new();
    let mut jumps: Vec<(usize, isize)> = Vec::new();
    let mut ip = 0usize;
    let mut saw_return = false;

    while ip < func.code.len() {
        starts.push(ip);
        let op_byte = func.code[ip];
        let Some(op) = decode_op(op_byte)
        else {
            return Err(BytecodeVerifyError::UnknownOpcode { func: func_index, offset: ip, op: op_byte });
        };
        let at = ip;
        ip += 1;

        match op {
            Op::LoadConst | Op::LoadGlobal | Op::StoreGlobal => {
                let idx = read_u16(func, func_index, &mut ip, at)?;
                if op == Op::LoadConst {
                    check_const(func, func_index, at, idx, func_count)?;
                }
                else if (idx as usize) >= func.const_names.len() && (idx as usize) >= func.consts.len() {
                    // LoadGlobal / StoreGlobal 用 const_names 槽。
                    if (idx as usize) >= func.const_names.len() {
                        return Err(BytecodeVerifyError::ConstOob { func: func_index, offset: at, index: idx, len: func.const_names.len() });
                    }
                }
            }
            Op::LoadLocal | Op::StoreLocal => {
                let slot = read_u16(func, func_index, &mut ip, at)?;
                if slot >= func.locals {
                    return Err(BytecodeVerifyError::LocalOob { func: func_index, offset: at, slot, locals: func.locals });
                }
            }
            Op::LoadString | Op::GetField | Op::SetField => {
                let idx = read_u16(func, func_index, &mut ip, at)?;
                if (idx as usize) >= func.strings.len() {
                    return Err(BytecodeVerifyError::StringOob { func: func_index, offset: at, index: idx, len: func.strings.len() });
                }
            }
            Op::Jump | Op::JumpIfFalse | Op::JumpIfTrue => {
                let rel = read_i16(func, func_index, &mut ip, at)? as isize;
                let target = (ip as isize) + rel;
                jumps.push((at, target));
            }
            Op::Call | Op::Pop | Op::NewArray => {
                let _ = read_u8(func, func_index, &mut ip, at)?;
            }
            Op::CallNative | Op::Send | Op::CallHost => {
                let idx = read_u16(func, func_index, &mut ip, at)?;
                let _argc = read_u8(func, func_index, &mut ip, at)?;
                if op == Op::CallHost {
                    if let Some(len) = host_slot_count {
                        if u32::from(idx) >= len {
                            return Err(BytecodeVerifyError::HostSlotOob { func: func_index, offset: at, slot: idx, len });
                        }
                    }
                }
                else if op == Op::CallNative {
                    if host_slot_count.is_some() {
                        return Err(BytecodeVerifyError::ResidualCallNative { func: func_index, offset: at });
                    }
                    let in_strings = (idx as usize) < func.strings.len();
                    let in_natives = (idx as usize) < native_len;
                    if !in_strings && !in_natives {
                        return Err(BytecodeVerifyError::StringOob {
                            func: func_index,
                            offset: at,
                            index: idx,
                            len: func.strings.len().max(native_len),
                        });
                    }
                }
                else if (idx as usize) >= func.strings.len() {
                    return Err(BytecodeVerifyError::StringOob { func: func_index, offset: at, index: idx, len: func.strings.len() });
                }
            }
            Op::JitEnter => {
                let _ = read_u32(func, func_index, &mut ip, at)?;
            }
            Op::Return => {
                saw_return = true;
            }
            _ => {}
        }
    }

    let start_set: std::collections::HashSet<usize> = starts.iter().copied().collect();
    for (at, target) in jumps {
        if target < 0 || target as usize > func.code.len() {
            return Err(BytecodeVerifyError::JumpOutOfBounds { func: func_index, offset: at, target });
        }
        let t = target as usize;
        if t != func.code.len() && !start_set.contains(&t) {
            return Err(BytecodeVerifyError::JumpOffBoundary { func: func_index, offset: at, target: t });
        }
    }

    if !saw_return {
        return Err(BytecodeVerifyError::MissingReturn { func: func_index });
    }
    Ok(())
}

fn check_const(func: &FuncProto, func_index: usize, at: usize, idx: u16, func_count: u32) -> Result<(), BytecodeVerifyError> {
    if (idx as usize) >= func.consts.len() {
        return Err(BytecodeVerifyError::ConstOob { func: func_index, offset: at, index: idx, len: func.consts.len() });
    }
    if let spark_gc::Value::Func(fidx) = func.consts[idx as usize] {
        if fidx >= func_count {
            return Err(BytecodeVerifyError::FuncOob { func: func_index, offset: at, index: fidx, len: func_count as usize });
        }
    }
    Ok(())
}

fn read_u8(func: &FuncProto, func_index: usize, ip: &mut usize, at: usize) -> Result<u8, BytecodeVerifyError> {
    if *ip >= func.code.len() {
        return Err(BytecodeVerifyError::TruncatedOperand { func: func_index, offset: at });
    }
    let v = func.code[*ip];
    *ip += 1;
    Ok(v)
}

fn read_u16(func: &FuncProto, func_index: usize, ip: &mut usize, at: usize) -> Result<u16, BytecodeVerifyError> {
    if *ip + 1 >= func.code.len() {
        return Err(BytecodeVerifyError::TruncatedOperand { func: func_index, offset: at });
    }
    let v = u16::from_le_bytes([func.code[*ip], func.code[*ip + 1]]);
    *ip += 2;
    Ok(v)
}

fn read_i16(func: &FuncProto, func_index: usize, ip: &mut usize, at: usize) -> Result<i16, BytecodeVerifyError> {
    Ok(read_u16(func, func_index, ip, at)? as i16)
}

fn read_u32(func: &FuncProto, func_index: usize, ip: &mut usize, at: usize) -> Result<u32, BytecodeVerifyError> {
    if *ip + 3 >= func.code.len() {
        return Err(BytecodeVerifyError::TruncatedOperand { func: func_index, offset: at });
    }
    let v = u32::from_le_bytes([func.code[*ip], func.code[*ip + 1], func.code[*ip + 2], func.code[*ip + 3]]);
    *ip += 4;
    Ok(v)
}
