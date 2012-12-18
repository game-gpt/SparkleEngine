//! 字节码结构验证（装载前）。
//!
//! 检查操作码、指令边界、跳转落点与常量/局部/字符串下标，不解释执行。

use crate::{FuncProto, Module, Op, decode_op};

/// 字节码验证错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeVerifyError {
    EmptyModule,
    EntryOutOfBounds { entry: usize, len: usize },
    EmptyFunction { index: usize },
    UnknownOpcode { func: usize, offset: usize, op: u8 },
    TruncatedOperand { func: usize, offset: usize },
    JumpOffBoundary { func: usize, offset: usize, target: usize },
    JumpOutOfBounds { func: usize, offset: usize, target: isize },
    LocalOob { func: usize, offset: usize, slot: u16, locals: u16 },
    ConstOob { func: usize, offset: usize, index: u16, len: usize },
    StringOob { func: usize, offset: usize, index: u16, len: usize },
    FuncOob { func: usize, offset: usize, index: u32, len: usize },
    HostSlotOob { func: usize, offset: usize, slot: u16, len: u32 },
    ResidualCallNative { func: usize, offset: usize },
    MissingReturn { func: usize },
}

impl BytecodeVerifyError {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FuncProto, Op};

    #[test]
    fn rejects_unknown_opcode() {
        let mut f = FuncProto::new("main", 0);
        f.code.push(255);
        let m = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
        let err = verify_bytecode(&m).unwrap_err();
        assert!(matches!(err, BytecodeVerifyError::UnknownOpcode { .. }));
    }

    #[test]
    fn rejects_jump_into_operand() {
        let mut f = FuncProto::new("main", 0);
        f.emit(Op::Jump);
        // 相对跳转到操作数中间：ip after i16 = 3, target = 3 + (-2) = 1（落在 i16 上）
        f.emit_i16(-2);
        f.emit(Op::LoadNull);
        f.emit(Op::Return);
        let m = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
        let err = verify_bytecode(&m).unwrap_err();
        assert!(matches!(err, BytecodeVerifyError::JumpOffBoundary { .. }));
    }

    #[test]
    fn accepts_simple_main() {
        let mut f = FuncProto::new("main", 0);
        f.emit(Op::LoadNull);
        f.emit(Op::Return);
        let m = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
        verify_bytecode(&m).unwrap();
    }

    #[test]
    fn rejects_host_slot_oob() {
        let mut f = FuncProto::new("main", 0);
        f.emit(Op::LoadNull);
        f.emit(Op::CallHost);
        f.emit_u16(99);
        f.emit_u8(1);
        f.emit(Op::Return);
        let m = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
        let err = verify_bytecode_with_host(&m, 2).unwrap_err();
        assert!(matches!(err, BytecodeVerifyError::HostSlotOob { slot: 99, len: 2, .. }));
    }

    #[test]
    fn sealed_rejects_residual_call_native() {
        let mut f = FuncProto::new("main", 0);
        let si = f.add_string("print");
        f.emit(Op::LoadNull);
        f.emit(Op::CallNative);
        f.emit_u16(si);
        f.emit_u8(1);
        f.emit(Op::Return);
        let m = Module { functions: vec![f], entry: 0, native_names: vec!["print".into()] };
        let err = verify_bytecode_with_host(&m, 1).unwrap_err();
        assert!(matches!(err, BytecodeVerifyError::ResidualCallNative { .. }));
    }
}
