//! RPG Maker / RGSS 风格 Ruby **子集** → `spark-vm` 字节码。
//!
//! `oak-ruby` 的 AST Builder 目前仍返回空语句列表，故本前端用手写递归下降
//! 覆盖常见战斗/事件脚本形态：`def` / 赋值 / `if` / `while` / 调用。
//! 归一执行层仍是 [`spark_vm::Module`]，与 Valkyrie、Lua 前端一致。

mod ast;
mod compile;
mod parse;

use spark_vm::Module;
use thiserror::Error;

pub use ast::RubyRoot;
pub use compile::compile_root;
pub use parse::parse as parse_source;

#[derive(Debug, Error)]
pub enum RubyScriptError {
    #[error("解析错误：{0}")]
    Parse(String),
    #[error("编译错误：{0}")]
    Compile(String),
}

/// 源码 → [`Module`]。
pub fn compile(source: &str, natives: &[&str]) -> Result<Module, RubyScriptError> {
    let root = parse_source(source).map_err(RubyScriptError::Parse)?;
    compile_root(&root, natives).map_err(RubyScriptError::Compile)
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
