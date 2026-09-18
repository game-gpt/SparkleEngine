//! Oaks `oak-lua` 解析 → `spark-vm` 字节码。
//!
//! 面向游戏脚本的 Lua 5.x **子集**（函数 / local / 控制流 / 算术）。
//! 完整语义（table、元表、协程）不在本前端范围。

mod compile;

use oak_core::{Builder, SourceText};
use oak_lua::{LuaBuilder, LuaLanguage, LuaRoot};
use spark_vm::Module;

pub use compile::compile_root;

#[derive(Debug)]
pub enum LuaScriptError {
    Parse { detail: String },
    Compile { detail: String },
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
    compile_root(&root, natives).map_err(|detail| LuaScriptError::Compile { detail })
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
        Err(e) => {
            let mut detail = e.to_string();
            if !out.diagnostics.is_empty() {
                let soft: Vec<String> = out.diagnostics.iter().map(|d| format!("{d:?}")).collect();
                detail.push_str("; ");
                detail.push_str(&soft.join("; "));
            }
            Err(LuaScriptError::Parse { detail })
        }
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
