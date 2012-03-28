//! Spark 脚本引擎：以 **Oaks (`oak-core`)** 管理源码，编译到 `spark-vm`。
//!
//! 语法刻意保持薄（`let` / `fn` / 表达式 / `print` / `return`），
//! 解析前端走 Oaks `SourceText`；日后可换成完整 `Language` + GreenTree。

mod compile;
mod lex;
mod parse;

use oak_core::source::SourceText;
use spark_jit::JitEngine;
use spark_vm::{HostHooks, Module, StdHost, Vm, VmError};
use thiserror::Error;

pub use compile::{compile_program, compile_program_with_natives};
pub use lex::{Lexer, Token, TokenKind};
pub use parse::{parse, Expr, Program, Stmt};

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("词法错误：{0}")]
    Lex(String),
    #[error("语法错误：{0}")]
    Parse(String),
    #[error("编译错误：{0}")]
    Compile(String),
    #[error(transparent)]
    Vm(#[from] VmError),
}

/// 脚本运行时：编译产物 + VM + 可选 JIT。
pub struct ScriptEngine {
    pub vm: Vm,
    pub jit: JitEngine,
}

impl ScriptEngine {
    pub fn compile(source: &str) -> Result<Self, ScriptError> {
        Self::compile_with_natives(source, &[])
    }

    pub fn compile_with_natives(source: &str, natives: &[&str]) -> Result<Self, ScriptError> {
        let text = SourceText::new(source);
        let src = text.text();
        let tokens = Lexer::new(src).tokenize().map_err(ScriptError::Lex)?;
        let program = parse(&tokens).map_err(ScriptError::Parse)?;
        let module =
            compile_program_with_natives(&program, natives).map_err(ScriptError::Compile)?;
        Ok(Self {
            vm: Vm::new(module),
            jit: JitEngine::new(256),
        })
    }

    pub fn from_module(module: Module) -> Self {
        Self {
            vm: Vm::new(module),
            jit: JitEngine::new(256),
        }
    }

    pub fn eval(&mut self) -> Result<spark_gc::Value, ScriptError> {
        let mut host = StdHost;
        self.eval_with(&mut host)
    }

    pub fn eval_with(&mut self, host: &mut dyn HostHooks) -> Result<spark_gc::Value, ScriptError> {
        let v = self.vm.run(host)?;
        let _ = self.jit.optimize_hot(&mut self.vm);
        Ok(v)
    }
}

/// 一键执行源码。
pub fn run(source: &str) -> Result<spark_gc::Value, ScriptError> {
    let mut eng = ScriptEngine::compile(source)?;
    eng.eval()
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_vm::HostHooks;

    struct Buf(String);
    impl HostHooks for Buf {
        fn print(&mut self, t: &str) {
            self.0.push_str(t);
            self.0.push(';');
        }
    }

    #[test]
    fn arithmetic() {
        let v = run("return 40 + 2").unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn let_and_print() {
        let mut eng = ScriptEngine::compile(
            r#"
            let x = 20 * 2
            print(x + 2)
            return x
            "#,
        )
        .unwrap();
        let mut buf = Buf(String::new());
        let v = eng.eval_with(&mut buf).unwrap();
        assert_eq!(v.as_number(), Some(40.0));
        assert!(buf.0.contains("42"));
    }

    #[test]
    fn function_call() {
        let v = run(
            r#"
            fn add(a, b) {
                return a + b
            }
            return add(40, 2)
            "#,
        )
        .unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }
}
