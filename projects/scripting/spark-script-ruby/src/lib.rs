//! RPG Maker / RGSS 风格 Ruby **子集** → `spark-vm` 字节码。
//!
//! `oak-ruby` 的 AST Builder 目前仍返回空语句列表，故本前端用手写递归下降
//! 覆盖常见战斗/事件脚本形态：`def` / 赋值 / `if` / `while` / 调用。
//! 归一执行层仍是 [`spark_vm::Module`]，与 Valkyrie、Lua 前端一致。

mod ast;
mod compile;
mod parse;

use compile::compile_root;
use parse::parse as parse_source;

use spark_diagnostics::{ErrorArg, ErrorArgs};
use spark_vm::Module;

pub use ast::RubyRoot;

#[derive(Debug)]
pub enum RubyScriptError {
    Parse { args: ErrorArgs },
    Compile { args: ErrorArgs },
}

impl RubyScriptError {
    pub fn parse_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Parse {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
        }
    }

    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
        }
    }

    pub fn parse_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::parse_reason(detail)
    }

    pub fn compile_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::compile_reason(detail)
    }
}

impl std::fmt::Display for RubyScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { .. } => f.write_str("spark.script.ruby.parse"),
            Self::Compile { .. } => f.write_str("spark.script.ruby.compile"),
        }
    }
}

impl std::error::Error for RubyScriptError {}

/// 源码 → [`Module`]。
pub fn compile(source: &str, natives: &[&str]) -> Result<Module, RubyScriptError> {
    let root = parse_source(source).map_err(RubyScriptError::parse_opaque)?;
    compile_root(&root, natives).map_err(RubyScriptError::compile_opaque)
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
    fn method_call() {
        let m = compile(
            r#"
            def add(a, b)
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
