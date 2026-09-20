//! 链接期：将 [`Op::CallNative`] 重写为 [`Op::CallHost`] 槽位调用。

use crate::{decode_op, FuncProto, Module, Op};

/// 按宿主短名表把模块内 `CallNative` 绑成 `CallHost`。
///
/// `slots[i]` 的下标即槽位。名字先查函数字符串池，再回退模块 `native_names`。
/// 未出现在 `slots` 中的 `CallNative` 保持不变（兼容过渡期）。
pub fn bind_host_slots(module: &mut Module, slots: &[&str]) -> Result<(), String> {
    let native_names = module.native_names.clone();
    for func in &mut module.functions {
        rewrite_func(func, slots, &native_names)?;
    }
    module.native_names = slots.iter().map(|s| (*s).to_string()).collect();
    Ok(())
}

fn rewrite_func(func: &mut FuncProto, slots: &[&str], native_names: &[String]) -> Result<(), String> {
    let mut ip = 0usize;
    while ip < func.code.len() {
        let at = ip;
        let op_byte = func.code[ip];
        let Some(op) = decode_op(op_byte) else {
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
                let name_idx = read_u16_at(func, ip, at)?;
                let _argc = read_u8_at(func, ip + 2, at)?;
                let name = func
                    .strings
                    .get(name_idx as usize)
                    .map(String::as_str)
                    .or_else(|| native_names.get(name_idx as usize).map(String::as_str));
                if let Some(name) = name {
                    if let Some(slot) = slots.iter().position(|s| *s == name) {
                        if slot > u16::MAX as usize {
                            return Err(format!("host_slot_overflow:{slot}"));
                        }
                        func.code[at] = Op::CallHost as u8;
                        let slot_u = slot as u16;
                        func.code[ip] = (slot_u & 0xff) as u8;
                        func.code[ip + 1] = (slot_u >> 8) as u8;
                    }
                }
                ip += 3;
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

fn read_u8_at(func: &FuncProto, ip: usize, at: usize) -> Result<u8, String> {
    if ip >= func.code.len() {
        return Err(format!("truncated_operand@{at}"));
    }
    Ok(func.code[ip])
}

fn read_u16_at(func: &FuncProto, ip: usize, at: usize) -> Result<u16, String> {
    if ip + 1 >= func.code.len() {
        return Err(format!("truncated_operand@{at}"));
    }
    Ok(u16::from_le_bytes([func.code[ip], func.code[ip + 1]]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StdHost, Vm};
    use spark_gc::Value;

    #[test]
    fn rewrites_call_native_to_call_host() {
        let mut f = FuncProto::new("__main", 0);
        let c = f.add_const_number(7.0);
        f.emit(Op::LoadConst);
        f.emit_u16(c);
        let si = f.add_string("inc");
        f.emit(Op::CallNative);
        f.emit_u16(si);
        f.emit_u8(1);
        f.emit(Op::Return);
        let mut module = Module {
            functions: vec![f],
            entry: 0,
            native_names: vec!["inc".into()],
        };
        bind_host_slots(&mut module, &["inc"]).unwrap();
        assert!(module.functions[0]
            .code
            .iter()
            .any(|&b| b == Op::CallHost as u8));
        assert!(!module.functions[0]
            .code
            .iter()
            .any(|&b| b == Op::CallNative as u8));

        let mut vm = Vm::new(module);
        vm.prepare_host_slots(["inc"]);
        vm.register_native("inc", |_ctx, args| {
            let n = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
            Ok(Value::Number(n + 1.0))
        });
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(8.0));
    }
}
