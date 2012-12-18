//! 链接期校验：正式字节码不得残留 [`Op::CallNative`]。
//!
//! 宿主调用须在 codegen 阶段按 [`Op::CallHost`] 槽位发射；链接只校验、不重写。

use crate::{FuncProto, Module, Op, decode_op};

/// 扫描模块：发现任何 `CallNative` 即失败。
pub fn reject_residual_call_native(module: &Module) -> Result<(), String> {
    for (fi, func) in module.functions.iter().enumerate() {
        scan_func(func, fi)?;
    }
    Ok(())
}

fn scan_func(func: &FuncProto, fi: usize) -> Result<(), String> {
    let mut ip = 0usize;
    while ip < func.code.len() {
        let at = ip;
        let op_byte = func.code[ip];
        let Some(op) = decode_op(op_byte)
        else {
            return Err(format!("unknown_opcode:{op_byte}@{at}"));
        };
        ip += 1;
        match op {
            Op::LoadConst
            | Op::LoadLocal
            | Op::StoreLocal
            | Op::LoadGlobal
            | Op::StoreGlobal
            | Op::Jump
            | Op::JumpIfFalse
            | Op::JumpIfTrue
            | Op::LoadString
            | Op::GetField
            | Op::SetField => {
                ip = skip(func, ip, 2, at)?;
            }
            Op::Call | Op::Pop | Op::NewArray => {
                ip = skip(func, ip, 1, at)?;
            }
            Op::JitEnter => {
                ip = skip(func, ip, 4, at)?;
            }
            Op::CallNative => {
                return Err(format!("residual_call_native:func_{fi}@{at}"));
            }
            Op::Send | Op::CallHost => {
                ip = skip(func, ip, 3, at)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn skip(func: &FuncProto, ip: usize, n: usize, at: usize) -> Result<usize, String> {
    if ip + n > func.code.len() {
        return Err(format!("truncated_operand@{at}"));
    }
    Ok(ip + n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_call_host_only() {
        let mut f = FuncProto::new("on_load", 0);
        f.emit(Op::CallHost);
        f.emit_u16(0);
        f.emit_u8(0);
        f.emit(Op::Return);
        let module = Module { functions: vec![f], entry: 0, native_names: vec!["inc".into()] };
        reject_residual_call_native(&module).unwrap();
    }

    #[test]
    fn rejects_call_native() {
        let mut f = FuncProto::new("on_load", 0);
        let si = f.add_string("inc");
        f.emit(Op::CallNative);
        f.emit_u16(si);
        f.emit_u8(0);
        f.emit(Op::Return);
        let module = Module { functions: vec![f], entry: 0, native_names: Vec::new() };
        let err = reject_residual_call_native(&module).unwrap_err();
        assert!(err.contains("residual_call_native"), "{err}");
    }
}
