//! Spark 脚本编译门面与运行时。
//!
//! 分层：
//! - [`ScriptCompiler`]：编译 → [`SparkObject`] / [`LinkedProgram`] / [`ExecutableImage`]
//! - [`ScriptRuntime`]：装载映像并执行（持有 VM / JIT）
//! - 语言前端经公共 IR 降低，不得在正式路径直接发射 `spark-vm::Op`

mod artifact;
mod cache;
mod codec;
mod compiler;
mod dep_graph;
mod diagnostic;
mod host_schema;
mod request;
mod runtime;
mod spko;
mod spkx;

use spark_diagnostics::{ErrorArg, ErrorArgs, ErrorContext, SourceSpan};
use spark_vm::{Module, VmError};

pub use artifact::{
    ExecutableImage, LinkError, LinkedProgram, SparkObject, VerifyError, ARTIFACT_FORMAT_VERSION,
};
pub use codec::{ArtifactIoError, SPKO_MAGIC, SPKX_MAGIC};
pub use cache::ArtifactCache;
pub use compiler::{CompiledPackage, ScriptCompiler};
pub use dep_graph::{DepGraphError, PackageDepGraph, PackageNode};
pub use diagnostic::{DiagnosticBatch, ScriptDiagnostic};
pub use host_schema::{
    compile_policy_from_request, CapabilityId, DeterminismClass, HostEffect, HostErrorModel,
    HostFunction, HostFunctionId, HostPhase, HostSchema, SuspensionBehavior, ThreadAffinity,
};
pub use request::{
    CompilationRequest, DebugInfoLevel, LanguageFrontend, LanguageProfile, LanguageProfileId,
    OptimizationLevel, PackageId, SourceFile,
};
pub use runtime::ScriptRuntime;
pub use spark_ir::{
    emit_module, emit_module_with_host, lower_module, BasicBlock, DeterminismKind, HirBinaryOp,
    HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp, HostBindEntry, HostBindTable,
    HostCompilePolicy, HostEffectKind, HostEmitMode, HostId, HostPhaseKind, HostRef, MirFunction,
    MirInst, MirModule, MirTerminator, MirValue, SymbolId, Ty,
};
pub use spark_ir::PackageId as IrPackageId;
pub use spark_script_valkyrie::{NativeParam, TypeRef};
pub use spark_vm::{
    reject_residual_call_native, verify_bytecode, verify_bytecode_with_host, BytecodeVerifyError,
};

/// 脚本源语言（便利枚举；配置面请用 [`LanguageProfile`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptLanguage {
    /// Oaks Valkyrie（默认）。
    Valkyrie,
    /// Spark Lua profile（`spark-lua-1`），不是完整 Lua 运行时声明。
    Lua,
    /// Spark Ruby profile（`spark-ruby-1`）；RGSS 应使用 `rgss-compat` profile。
    Ruby,
}

/// 脚本管线阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptStage {
    Parse,
    Compile,
    Link,
    Verify,
    Runtime,
}

/// 结构化脚本错误（码 + 参数；`Display` 只输出稳定码）。
#[derive(Debug)]
pub enum ScriptError {
    Parse {
        args: ErrorArgs,
        span: Option<SourceSpan>,
    },
    Compile {
        args: ErrorArgs,
        span: Option<SourceSpan>,
    },
    Vm(VmError),
}

impl ScriptError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Parse { .. } => "spark.script.parse",
            Self::Compile { .. } => "spark.script.compile",
            Self::Vm(e) => e.code(),
        }
    }

    pub fn stage(&self) -> ScriptStage {
        match self {
            Self::Parse { .. } => ScriptStage::Parse,
            Self::Compile { .. } => ScriptStage::Compile,
            Self::Vm(_) => ScriptStage::Runtime,
        }
    }

    pub fn parse_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Parse {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
            span: None,
        }
    }

    pub fn parse_at(reason: impl Into<std::sync::Arc<str>>, span: SourceSpan) -> Self {
        Self::Parse {
            args: ErrorArgs::new()
                .with("reason", ErrorArg::String(reason.into()))
                .with("span", ErrorArg::Span(span)),
            span: Some(span),
        }
    }

    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
            span: None,
        }
    }

    pub fn parse_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::parse_reason(detail)
    }

    pub fn compile_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::compile_reason(detail)
    }

    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::Parse { span, .. } | Self::Compile { span, .. } => *span,
            Self::Vm(_) => None,
        }
    }

    pub fn context(&self) -> ErrorContext {
        let mut ctx = ErrorContext::new().target("spark-script");
        if let Some(span) = self.span() {
            ctx = ctx.with_span(span);
        }
        ctx
    }

    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Parse { args, .. } | Self::Compile { args, .. } => args.clone(),
            Self::Vm(e) => e.args(),
        }
    }
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ScriptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Vm(e) => Some(e),
            _ => None,
        }
    }
}

impl From<VmError> for ScriptError {
    fn from(value: VmError) -> Self {
        Self::Vm(value)
    }
}

impl From<spark_script_valkyrie::ValkyrieScriptError> for ScriptError {
    fn from(e: spark_script_valkyrie::ValkyrieScriptError) -> Self {
        match e {
            spark_script_valkyrie::ValkyrieScriptError::Parse { args, span } => {
                ScriptError::Parse { args, span }
            }
            spark_script_valkyrie::ValkyrieScriptError::Compile { args, span } => {
                ScriptError::Compile { args, span }
            }
        }
    }
}

impl From<spark_script_lua::LuaScriptError> for ScriptError {
    fn from(e: spark_script_lua::LuaScriptError) -> Self {
        match e {
            spark_script_lua::LuaScriptError::Parse { args } => {
                ScriptError::Parse { args, span: None }
            }
            spark_script_lua::LuaScriptError::Compile { args } => {
                ScriptError::Compile { args, span: None }
            }
        }
    }
}

impl From<spark_script_ruby::RubyScriptError> for ScriptError {
    fn from(e: spark_script_ruby::RubyScriptError) -> Self {
        match e {
            spark_script_ruby::RubyScriptError::Parse { args } => {
                ScriptError::Parse { args, span: None }
            }
            spark_script_ruby::RubyScriptError::Compile { args } => {
                ScriptError::Compile { args, span: None }
            }
        }
    }
}

/// 仅编译为 [`Module`]（不建 VM；供前端单测）。
pub fn compile_module(
    language: ScriptLanguage,
    source: &str,
    hosts: &HostBindTable,
) -> Result<Module, ScriptError> {
    match language {
        ScriptLanguage::Valkyrie => Ok(spark_script_valkyrie::compile_with_binds(source, hosts)?),
        ScriptLanguage::Lua => Ok(spark_script_lua::compile_with_binds(source, hosts)?),
        ScriptLanguage::Ruby => Ok(spark_script_ruby::compile_with_binds(source, hosts)?),
    }
}

/// 调试：列出 Valkyrie 根上 `micro` 名。
pub fn list_micros(source: &str) -> Result<Vec<String>, ScriptError> {
    Ok(spark_script_valkyrie::list_micros(source)?)
}


#[cfg(test)]
mod tests {
    use super::*;
    use spark_vm::StdHost;

    fn eval_source(language: ScriptLanguage, source: &str) -> spark_gc::Value {
        let host = HostSchema::new(1);
        let package = ScriptCompiler::new()
            .compile_source(language, source, &host)
            .unwrap();
        let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
        rt.call_on_load_std().unwrap()
    }

    #[test]
    fn valkyrie_arithmetic() {
        let v = eval_source(ScriptLanguage::Valkyrie, "return 40 + 2");
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn lua_function() {
        let v = eval_source(
            ScriptLanguage::Lua,
            r#"
            function add(a, b)
              return a + b
            end
            return add(40, 2)
            "#,
        );
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn ruby_method() {
        let v = eval_source(
            ScriptLanguage::Ruby,
            "def add(a, b)\n  return a + b\nend\nreturn add(40, 2)\n",
        );
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn call_micro_from_host() {
        let host = HostSchema::new(1);
        let package = ScriptCompiler::new()
            .compile_source(
                ScriptLanguage::Valkyrie,
                r#"
                micro add(a, b) {
                    return a + b
                }
                return 0
                "#,
                &host,
            )
            .unwrap();
        let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
        let v = rt
            .call("add", &[spark_gc::Value::Number(40.0), spark_gc::Value::Number(2.0)], &mut StdHost)
            .unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn host_schema_compile_and_call() {
        let mut host = HostSchema::new(1);
        host.insert(HostFunction::new(HostFunctionId::new("host", "ping", 1)));
        let package = ScriptCompiler::new()
            .compile_source(ScriptLanguage::Valkyrie, "return ping()", &host)
            .unwrap();
        let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
        rt.vm.register_native("host.ping", |_ctx, _args| Ok(spark_gc::Value::Number(7.0)));
        let v = rt.call_on_load_std().unwrap();
        assert_eq!(v.as_number(), Some(7.0));
    }

    #[test]
    fn lua_and_ruby_host_schema_compile() {
        let mut host = HostSchema::new(1);
        host.insert(HostFunction::new(HostFunctionId::new("host", "ping", 1)));
        for lang in [ScriptLanguage::Lua, ScriptLanguage::Ruby] {
            let source = match lang {
                ScriptLanguage::Lua => "return ping(1)",
                ScriptLanguage::Ruby => "return ping(1)",
                ScriptLanguage::Valkyrie => unreachable!(),
            };
            let package = ScriptCompiler::new()
                .compile_source(lang, source, &host)
                .unwrap();
            let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
            rt.vm.register_native("host.ping", |_ctx, args| {
                Ok(args.first().cloned().unwrap_or(spark_gc::Value::Null))
            });
            let v = rt.call_on_load_std().unwrap();
            assert_eq!(v.as_number(), Some(1.0));
        }
    }

    #[test]
    fn valkyrie_parse_error_propagates_span() {
        let err = compile_module(ScriptLanguage::Valkyrie, "@@@", &HostBindTable::new())
            .expect_err("bare attributes");
        assert_eq!(err.code(), "spark.script.parse");
    }

    #[test]
    fn compiler_produces_executable_image() {
        let host = HostSchema::new(1);
        let package = ScriptCompiler::new()
            .compile_source(ScriptLanguage::Valkyrie, "return 1 + 2", &host)
            .unwrap();
        assert!(package.image.module().functions.iter().any(|f| f.name == "on_load"));
        let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
        assert_eq!(rt.call_on_load_std().unwrap().as_number(), Some(3.0));
    }

    #[test]
    fn runtime_host_slot_call_executes_registered_native() {
        let mut host = HostSchema::new(1);
        host.insert(HostFunction::new(HostFunctionId::new("host", "ping", 1)));
        let package = ScriptCompiler::new()
            .compile_source(ScriptLanguage::Valkyrie, "return ping()", &host)
            .unwrap();
        let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
        rt.vm.register_native("host.ping", |_ctx, _args| Ok(spark_gc::Value::Number(9.0)));
        assert_eq!(rt.call_on_load_std().unwrap().as_number(), Some(9.0));
    }
}
