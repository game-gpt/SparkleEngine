//! Oaks `oak-lua` 解析 → `spark-vm` 字节码。
//!
//! 面向游戏脚本的 Lua 5.x **子集**（函数 / local / 控制流 / 算术）。
//! 完整语义（table、元表、协程）不在本前端范围。

mod compile;

use compile::compile_root;

use oak_core::{Builder, SourceText};
use oak_lua::{LuaBuilder, LuaLanguage, LuaRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs};
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

    /// 兼容旧调用：仍写入 `reason`，禁止把 oak Display 句子塞进 opaque。
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

/// 源码 → [`Module`]。
pub fn compile(source: &str, natives: &[&str]) -> Result<Module, LuaScriptError> {
    let root = parse(source)?;
    compile_root(&root, natives).map_err(LuaScriptError::compile_opaque)
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
    use spark_vm::{StdHost, Vm};

    #[test]
    fn arithmetic_main() {
        let m = compile("return 40 + 2", &[]).unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn function_call() {
        let m = compile(
            r#"
            function add(a, b)
              return a + b
            end
            return add(40, 2)
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }
}
