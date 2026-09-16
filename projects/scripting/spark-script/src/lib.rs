//! Spark 脚本引擎门面：多语言前端归一到栈式 [`spark_vm`]。
//!
//! | 前端 | Crate | 解析 |
//! |------|-------|------|
//! | Valkyrie | `spark-script-valkyrie` | Oaks `oak-valkyrie` |
//! | Lua | `spark-script-lua` | Oaks `oak-lua` |
//! | Ruby（RPG Maker / RGSS 子集） | `spark-script-ruby` | 自研子集（上游 Builder 未就绪） |
//!
//! 游戏绑定经原生函数表注入。ECS 侧用 [`spark_vm::Vm::call_function`] 调脚本，
//! 不把 World 塞进本 crate。

use spark_jit::JitEngine;
use spark_vm::{HostHooks, Module, StdHost, Vm, VmError};
use thiserror::Error;

/// 脚本源语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptLanguage {
    /// Oaks Valkyrie（默认）。
    Valkyrie,
    /// Lua 5.x 子集。
    Lua,
    /// RPG Maker / RGSS 风格 Ruby 子集。
    Ruby,
}

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("解析错误：{0}")]
    Parse(String),
    #[error("编译错误：{0}")]
    Compile(String),
    #[error(transparent)]
    Vm(#[from] VmError),
}

impl From<spark_script_valkyrie::ValkyrieScriptError> for ScriptError {
    fn from(e: spark_script_valkyrie::ValkyrieScriptError) -> Self {
        match e {
            spark_script_valkyrie::ValkyrieScriptError::Parse(s) => ScriptError::Parse(s),
            spark_script_valkyrie::ValkyrieScriptError::Compile(s) => ScriptError::Compile(s),
        }
    }
}

impl From<spark_script_lua::LuaScriptError> for ScriptError {
    fn from(e: spark_script_lua::LuaScriptError) -> Self {
        match e {
            spark_script_lua::LuaScriptError::Parse(s) => ScriptError::Parse(s),
            spark_script_lua::LuaScriptError::Compile(s) => ScriptError::Compile(s),
        }
    }
}

impl From<spark_script_ruby::RubyScriptError> for ScriptError {
    fn from(e: spark_script_ruby::RubyScriptError) -> Self {
        match e {
            spark_script_ruby::RubyScriptError::Parse(s) => ScriptError::Parse(s),
            spark_script_ruby::RubyScriptError::Compile(s) => ScriptError::Compile(s),
        }
    }
}

/// 脚本运行时：模块 + VM + JIT 热度特化。
pub struct ScriptEngine {
    pub vm: Vm,
    pub jit: JitEngine,
    pub language: ScriptLanguage,
}

impl ScriptEngine {
    /// 默认按 Valkyrie 编译。
    pub fn compile(source: &str) -> Result<Self, ScriptError> {
        Self::compile_with(ScriptLanguage::Valkyrie, source, &[])
    }

    pub fn compile_with_natives(source: &str, natives: &[&str]) -> Result<Self, ScriptError> {
        Self::compile_with(ScriptLanguage::Valkyrie, source, natives)
    }

    /// 指定前端语言编译到同一 [`Module`] / VM。
    pub fn compile_with(
        language: ScriptLanguage,
        source: &str,
        natives: &[&str],
    ) -> Result<Self, ScriptError> {
        let module = compile_module(language, source, natives)?;
        Ok(Self {
            vm: Vm::new(module),
            jit: JitEngine::new(256),
            language,
        })
    }

    pub fn from_module(module: Module) -> Self {
        Self {
            vm: Vm::new(module),
            jit: JitEngine::new(256),
            language: ScriptLanguage::Valkyrie,
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

    /// 供 ECS System 调用命名函数（Valkyrie `micro` / Lua `function` / Ruby `def`）。
    pub fn call(
        &mut self,
        name: &str,
        args: &[spark_gc::Value],
        host: &mut dyn HostHooks,
    ) -> Result<spark_gc::Value, ScriptError> {
        let v = self.vm.call_function(name, args, host)?;
        let _ = self.jit.optimize_hot(&mut self.vm);
        Ok(v)
    }
}

/// 仅编译为 [`Module`]（不建 VM）。
pub fn compile_module(
    language: ScriptLanguage,
    source: &str,
    natives: &[&str],
) -> Result<Module, ScriptError> {
    match language {
        ScriptLanguage::Valkyrie => Ok(spark_script_valkyrie::compile(source, natives)?),
        ScriptLanguage::Lua => Ok(spark_script_lua::compile(source, natives)?),
        ScriptLanguage::Ruby => Ok(spark_script_ruby::compile(source, natives)?),
    }
}

/// 一键执行源码（默认 Valkyrie，跑 `__main`）。
pub fn run(source: &str) -> Result<spark_gc::Value, ScriptError> {
    let mut eng = ScriptEngine::compile(source)?;
    eng.eval()
}

/// 指定语言一键执行。
pub fn run_with(language: ScriptLanguage, source: &str) -> Result<spark_gc::Value, ScriptError> {
    let mut eng = ScriptEngine::compile_with(language, source, &[])?;
    eng.eval()
}

/// 调试：列出 Valkyrie 根上 `micro` 名。
pub fn list_micros(source: &str) -> Result<Vec<String>, ScriptError> {
    Ok(spark_script_valkyrie::list_micros(source)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Buf(String);
    impl HostHooks for Buf {
        fn print(&mut self, t: &str) {
            self.0.push_str(t);
            self.0.push(';');
        }
    }

    #[test]
    fn valkyrie_arithmetic() {
        let v = run("return 40 + 2").unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn lua_function() {
        let v = run_with(
            ScriptLanguage::Lua,
            r#"
            function add(a, b)
              return a + b
            end
            return add(40, 2)
            "#,
        )
        .unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn ruby_method() {
        let v = run_with(
            ScriptLanguage::Ruby,
            r#"
            def add(a, b)
              return a + b
            end
            return add(40, 2)
            "#,
        )
        .unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn call_micro_from_host() {
        let mut eng = ScriptEngine::compile(
            r#"
            micro double(x) {
                return x * 2
            }
            "#,
        )
        .unwrap();
        let mut host = Buf(String::new());
        let v = eng
            .call("double", &[spark_gc::Value::Number(21.0)], &mut host)
            .unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }
}
