//! Oaks `oak-lua` 解析 → 公共 IR → `spark-vm` 字节码。
//!
//! 面向游戏脚本的 Lua 5.x **子集**（函数 / local / 控制流 / 算术）。
//! 完整语义（table、元表、协程）不在本前端范围。
//! 正式编译只经 `spark-script-ir`；不支持的构造必须报错。

mod lower;

use lower::lower_root_to_hir;

use oak_core::{Builder, SourceText};
use oak_lua::{LuaBuilder, LuaLanguage, LuaRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs};
use spark_script_ir::{
    emit_module_with_host, lower_module, HostBindTable, HostEmitMode,
};
use spark_vm::Module;

#[derive(Debug)]
pub enum LuaScriptError {
    Parse { args: ErrorArgs },
    Compile { args: ErrorArgs },
}

impl LuaScriptError {
    pub fn parse_failed(diag_count: u64) -> Self {
        Self::Parse {
            args: ErrorArgs::new()
                .with("reason", ErrorArg::String(std::sync::Arc::from("parse_failed")))
                .with("diagnostics", ErrorArg::Unsigned(diag_count)),
        }
    }

    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
        }
    }

    /// 写入 `reason` 参数（机器令牌，非 Locale 句子）。
    pub fn compile_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::compile_reason(detail)
    }
}

impl std::fmt::Display for LuaScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { .. } => f.write_str("spark.script.lua.parse"),
            Self::Compile { .. } => f.write_str("spark.script.lua.compile"),
        }
    }
}

impl std::error::Error for LuaScriptError {}

/// 无宿主绑定的源码 → [`Module`]。有宿主时请用 [`compile_with_binds`]。
pub fn compile(source: &str) -> Result<Module, LuaScriptError> {
    compile_with_binds(source, &HostBindTable::new())
}

/// 带完整宿主绑定表的编译入口。
pub fn compile_with_binds(
    source: &str,
    hosts: &HostBindTable,
) -> Result<Module, LuaScriptError> {
    let root = parse(source)?;
    let hir = lower_root_to_hir(&root, hosts).map_err(LuaScriptError::compile_opaque)?;
    let mir = lower_module(&hir).map_err(LuaScriptError::compile_opaque)?;
    let mode = if hosts.is_empty() {
        HostEmitMode::NoHost
    } else {
        HostEmitMode::Bound(hosts)
    };
    emit_module_with_host(&mir, mode).map_err(LuaScriptError::compile_opaque)
}

/// 解析为 AST 根。
pub fn parse(source: &str) -> Result<LuaRoot, LuaScriptError> {
    let language = LuaLanguage::default();
    let builder = LuaBuilder::new(&language);
    let text = SourceText::new(source);
    let mut session = oak_core::ParseSession::<LuaLanguage>::default();
    let out = builder.build(&text, &[], &mut session);
    match out.result {
        Ok(root) => Ok(root),
        Err(_e) => Err(LuaScriptError::parse_failed(out.diagnostics.len() as u64)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_vm::{Op, StdHost, Vm};

    #[test]
    fn unsupported_table_literal_fails() {
        // table 构造不在 IR 子集；必须明确失败，必须明确失败。
        let err = compile("return {a=1}").unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("ir_unsupported") || msg.contains("unsupported") || msg.contains("reason"),
            "{msg}"
        );
    }

    #[test]
    fn arithmetic_main() {
        let m = compile("return 40 + 2").unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn arithmetic_via_ir_has_no_call_native() {
        let m = compile("return 40 + 2").unwrap();
        assert!(m.functions.iter().all(|f| {
            !f.code.iter().any(|&b| b == Op::CallNative as u8 || b == Op::CallHost as u8)
        }));
    }

    #[test]
    fn local_and_if_via_ir() {
        let m = compile(
            r#"
            local x = 1
            if x < 2 then
                return 42
            else
                return 0
            end
            "#)
        .unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn elseif_and_do_via_ir() {
        let m = compile(
            r#"
            local x = 2
            if x < 1 then
                return 0
            elseif x < 3 then
                return 42
            else
                return 1
            end
            "#)
        .unwrap();
        let mut vm = Vm::new(m);
        assert_eq!(vm.run(&mut StdHost).unwrap().as_number(), Some(42.0));

        let m = compile(
            r#"
            local n = 0
            do
                n = n + 40
                n = n + 2
            end
            return n
            "#)
        .unwrap();
        let mut vm = Vm::new(m);
        assert_eq!(vm.run(&mut StdHost).unwrap().as_number(), Some(42.0));
    }

    #[test]
    fn repeat_until_via_ir() {
        let m = compile(
            r#"
            local n = 0
            repeat
                n = n + 1
            until n >= 3
            return n
            "#)
        .unwrap();
        let mut vm = Vm::new(m);
        assert_eq!(vm.run(&mut StdHost).unwrap().as_number(), Some(3.0));
    }

    #[test]
    fn and_or_via_ir() {
        let m = compile(
            r#"
            local a = 0
            local b = 7
            if a and b then
                return 1
            end
            if a or b then
                return 42
            end
            return 0
            "#)
        .unwrap();
        let mut vm = Vm::new(m);
        assert_eq!(vm.run(&mut StdHost).unwrap().as_number(), Some(42.0));
    }

    #[test]
    fn function_call() {
        let m = compile(
            r#"
            function double(n)
                return n * 2
            end
            return double(21)
            "#)
        .unwrap();
        assert!(
            m.functions.iter().any(|f| f.name == "double"),
            "expected IR function proto"
        );
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn while_via_ir() {
        let m = compile(
            r#"
            local n = 0
            while n < 3 do
                n = n + 1
            end
            return n
            "#)
        .unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(3.0));
    }

    #[test]
    fn host_call_via_ir() {
        use spark_gc::Value;
        let m = compile_with_binds("return ping(7)", &HostBindTable::from_short_names(&["ping"]).unwrap()).unwrap();
        assert!(m
            .functions
            .iter()
            .any(|f| f.code.iter().any(|&b| b == Op::CallHost as u8)));
        let mut vm = Vm::new(m);
        vm.prepare_host_slots(["ping"]);
        vm.register_native("ping", |_ctx, args| {
            let n = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
            Ok(Value::Number(n + 1.0))
        });
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(8.0));
    }
}
