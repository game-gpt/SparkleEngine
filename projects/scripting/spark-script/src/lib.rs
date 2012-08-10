//! Spark 脚本编译门面与过渡期运行包装。
//!
//! **目标分层**（见工作区规划）：
//! - [`ScriptCompiler`]：编译 → [`SparkObject`] / [`LinkedProgram`] / [`ExecutableImage`]
//! - [`ScriptRuntime`]：装载映像并执行（持有 VM / JIT）
//! - 语言前端最终只降低到公共 IR，不得直接发射 `spark-vm::Op`
//!
//! **过渡期**：[`ScriptEngine`] 仍保留给 `spark-engine` 等调用方，内部改为
//! 编译器 + 运行时组合；前端仍直接产出 [`spark_vm::Module`]。

mod artifact;
mod cache;
mod compiler;
mod diagnostic;
mod host_schema;
mod request;
mod runtime;

use spark_diagnostics::{ErrorArg, ErrorArgs, ErrorContext, SourceSpan};
use spark_vm::{HostHooks, Module, StdHost, VmError};

pub use artifact::{
    ExecutableImage, LinkError, LinkedProgram, SparkObject, VerifyError, ARTIFACT_FORMAT_VERSION,
};
pub use cache::ArtifactCache;
pub use compiler::{compile_package_with_registry, CompiledPackage, ScriptCompiler};
pub use diagnostic::{DiagnosticBatch, ScriptDiagnostic};
pub use host_schema::{
    CapabilityId, DeterminismClass, HostEffect, HostErrorModel, HostFunction, HostFunctionId,
    HostPhase, HostSchema, SuspensionBehavior, ThreadAffinity,
};
pub use request::{
    CompilationRequest, DebugInfoLevel, LanguageFrontend, LanguageProfile, LanguageProfileId,
    OptimizationLevel, PackageId, SourceFile,
};
pub use runtime::ScriptRuntime;
pub use spark_script_ir::{
    BasicBlock, HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp, HostEmitMode,
    HostRef, MirFunction, MirInst, MirModule, MirTerminator, MirValue, SymbolId, Ty, emit_module,
    emit_module_with_host, lower_module,
};
pub use spark_script_ir::PackageId as IrPackageId;
pub use spark_script_valkyrie::{NativeParam, NativeRegistry, NativeSignature, TypeRef};
pub use spark_vm::{bind_host_slots, verify_bytecode, BytecodeVerifyError};

/// 脚本源语言（过渡期枚举；正式路径请用 [`LanguageProfile`]）。
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

/// 过渡期门面：编译 + 运行揉在一起。新代码请拆用 [`ScriptCompiler`] / [`ScriptRuntime`]。
///
/// 字段仍公开以兼容 `spark-engine` 插件安装路径；语义上 `vm`/`jit` 属于运行时。
pub struct ScriptEngine {
    pub vm: spark_vm::Vm,
    pub jit: spark_jit::JitEngine,
    pub language: ScriptLanguage,
}

impl ScriptEngine {
    fn from_runtime(rt: ScriptRuntime) -> Self {
        Self {
            vm: rt.vm,
            jit: rt.jit,
            language: rt.language.frontend.into(),
        }
    }

    /// 默认按 Valkyrie 编译。
    pub fn compile(source: &str) -> Result<Self, ScriptError> {
        Self::compile_with(ScriptLanguage::Valkyrie, source, &[])
    }

    pub fn compile_with_natives(source: &str, natives: &[&str]) -> Result<Self, ScriptError> {
        Self::compile_with(ScriptLanguage::Valkyrie, source, natives)
    }

    /// 使用完整宿主签名编译（经 [`HostSchema`]）。
    pub fn compile_with_registry(
        language: ScriptLanguage,
        source: &str,
        natives: &NativeRegistry,
    ) -> Result<Self, ScriptError> {
        let package = compile_package_with_registry(language, source, natives)?;
        let host = HostSchema::from_native_registry(natives);
        let runtime = ScriptRuntime::from_image(&package.image, &host)?;
        Ok(Self::from_runtime(runtime))
    }

    /// 指定前端语言编译。
    pub fn compile_with(
        language: ScriptLanguage,
        source: &str,
        natives: &[&str],
    ) -> Result<Self, ScriptError> {
        let package = ScriptCompiler::new().compile_with_native_names(language, source, natives)?;
        let host = stub_schema_from_names(natives);
        let runtime = ScriptRuntime::from_image(&package.image, &host)?;
        Ok(Self::from_runtime(runtime))
    }

    pub fn from_module(module: Module) -> Self {
        Self::from_runtime(ScriptRuntime::from_legacy_module(
            module,
            ScriptLanguage::Valkyrie,
        ))
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

    /// 供 ECS System 调用命名函数。
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

fn stub_schema_from_names(names: &[&str]) -> HostSchema {
    let mut schema = HostSchema::new(1);
    for name in names {
        schema.insert(HostFunction::new(HostFunctionId::new("host", *name, 1)));
    }
    schema
}

/// 仅编译为 [`Module`]（不建 VM）。过渡期 API。
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

/// 带 [`NativeRegistry`] 编译；非 Valkyrie 前端回退为仅函数名。
pub fn compile_module_with_registry(
    language: ScriptLanguage,
    source: &str,
    natives: &NativeRegistry,
) -> Result<Module, ScriptError> {
    match language {
        ScriptLanguage::Valkyrie => {
            Ok(spark_script_valkyrie::compile_with_registry(source, natives)?)
        }
        ScriptLanguage::Lua | ScriptLanguage::Ruby => {
            let names = natives.name_list();
            compile_module(language, source, &names)
        }
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

    #[test]
    fn compile_with_native_registry() {
        let mut reg = NativeRegistry::new();
        reg.insert(
            NativeSignature::new("ping")
                .param(NativeParam::new("n", "Number"))
                .returns("Number"),
        );
        let eng = ScriptEngine::compile_with_registry(
            ScriptLanguage::Valkyrie,
            "return ping(1)",
            &reg,
        )
        .unwrap();
        assert!(eng.vm.module.native_names.iter().any(|n| n == "ping"));
        assert_eq!(eng.vm.host_slot_names, vec!["ping".to_string()]);
        assert!(eng.vm.module.functions.iter().any(|f| {
            f.code.iter().any(|&b| b == spark_vm::Op::CallHost as u8)
        }));
    }

    #[test]
    fn runtime_host_slot_call_executes_registered_native() {
        let mut host_schema = HostSchema::new(1);
        host_schema.insert(HostFunction::new(HostFunctionId::new("host", "triple", 1)));
        let mut compiler = ScriptCompiler::new();
        let package = compiler
            .compile_source(
                ScriptLanguage::Valkyrie,
                "return triple(14)",
                &host_schema,
            )
            .unwrap();
        let mut rt = ScriptRuntime::from_image(&package.image, &host_schema).unwrap();
        rt.vm.register_native("triple", |_ctx, args| {
            let n = args
                .first()
                .and_then(|v| v.as_number())
                .unwrap_or(0.0);
            Ok(spark_gc::Value::Number(n * 3.0))
        });
        let v = rt.eval().unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn compiler_produces_executable_image() {
        let mut compiler = ScriptCompiler::new();
        let package = compiler
            .compile_with_native_names(ScriptLanguage::Valkyrie, "return 1 + 2", &[])
            .unwrap();
        assert_eq!(package.image.format_version, ARTIFACT_FORMAT_VERSION);
        let host = HostSchema::new(1);
        let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
        let v = rt.eval().unwrap();
        assert_eq!(v.as_number(), Some(3.0));
    }

    #[test]
    fn valkyrie_parse_error_propagates_span() {
        let err =
            compile_module(ScriptLanguage::Valkyrie, "@@@", &[]).expect_err("bare attributes");
        assert_eq!(err.code(), "spark.script.parse");
        assert!(err.span().is_some());
        assert!(matches!(
            err.args().get("reason"),
            Some(ErrorArg::String(s)) if s.as_ref() == "parse_failed"
        ));
    }
}
