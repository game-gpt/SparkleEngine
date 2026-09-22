//! Spark 脚本编译门面与运行时。
//!
//! 分层：
//! - [`ScriptCompiler`]：编译 → [`SparkObject`] / [`LinkedProgram`] / [`ExecutableImage`]
//! - [`ScriptRuntime`]：装载映像并执行（持有 VM / JIT）
//! - 语言前端经公共 IR 降低，不得在正式路径直接发射 `spark-vm::Op`

#![forbid(missing_docs)]
pub mod artifact;
pub mod cache;
pub mod codec;
pub mod compiler;
pub mod dep_graph;
pub mod diagnostic;
pub mod host_schema;
pub mod request;
pub mod runtime;
pub mod spko;
pub mod spkx;

use spark_diagnostics::{ErrorArg, ErrorArgs, ErrorContext, SourceSpan};
use spark_vm::{Module, VmError};

pub use artifact::{ARTIFACT_FORMAT_VERSION, ExecutableImage, LinkError, LinkedProgram, SparkObject, VerifyError};
pub use cache::ArtifactCache;
pub use codec::{ArtifactIoError, SPKO_MAGIC, SPKX_MAGIC};
pub use compiler::{CompiledPackage, ScriptCompiler};
pub use dep_graph::{DepGraphError, PackageDepGraph, PackageNode};
pub use diagnostic::{DiagnosticBatch, ScriptDiagnostic};
pub use host_schema::{
    CapabilityId, DeterminismClass, HostEffect, HostErrorModel, HostFunction, HostFunctionId, HostPhase, HostSchema, SuspensionBehavior,
    ThreadAffinity, compile_policy_from_request,
};
pub use request::{
    CompilationRequest, DebugInfoLevel, LanguageFrontend, LanguageProfile, LanguageProfileId, OptimizationLevel, PackageId, SourceFile,
};
pub use runtime::ScriptRuntime;
pub use spark_ir::{
    BasicBlock, DeterminismKind, HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp, HostBindEntry, HostBindTable,
    HostCompilePolicy, HostEffectKind, HostEmitMode, HostId, HostPhaseKind, HostRef, MirFunction, MirInst, MirModule, MirTerminator, MirValue,
    PackageId as IrPackageId, SymbolId, Ty, emit_module, emit_module_with_host, lower_module,
};
pub use spark_script_valkyrie::{NativeParam, TypeRef};
pub use spark_vm::{BytecodeVerifyError, reject_residual_call_native, verify_bytecode, verify_bytecode_with_host};

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
    /// 前端解析 / 词法语法。
    Parse,
    /// 前端降低、IR、宿主绑定与目标封装。
    Compile,
    /// 跨单元链接与宿主导入解析。
    Link,
    /// 字节码验证（含宿主槽位契约）。
    Verify,
    /// VM / JIT 执行期。
    Runtime,
}

/// 结构化脚本错误（码 + 参数；`Display` 只输出稳定码）。
#[derive(Debug)]
pub enum ScriptError {
    /// 解析失败；稳定码 `spark.script.parse`。
    Parse {
        /// 结构化参数（含 `reason`，可选 `span`）。
        args: ErrorArgs,
        /// 主定位跨度；无定位时为 `None`。
        span: Option<SourceSpan>,
    },
    /// 编译 / 链接 / 验证失败；稳定码 `spark.script.compile`。
    Compile {
        /// 结构化参数（含 `reason`，可选 `span`）。
        args: ErrorArgs,
        /// 主定位跨度；无定位时为 `None`。
        span: Option<SourceSpan>,
    },
    /// 运行期 VM 错误；码委托给 [`VmError::code`]。
    Vm(VmError),
}

impl ScriptError {
    /// 稳定错误码（面向诊断渲染与测试断言）。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Parse { .. } => "spark.script.parse",
            Self::Compile { .. } => "spark.script.compile",
            Self::Vm(e) => e.code(),
        }
    }

    /// 错误所属管线阶段。
    pub fn stage(&self) -> ScriptStage {
        match self {
            Self::Parse { .. } => ScriptStage::Parse,
            Self::Compile { .. } => ScriptStage::Compile,
            Self::Vm(_) => ScriptStage::Runtime,
        }
    }

    /// 构造无跨度的解析错误（`args.reason`）。
    pub fn parse_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Parse { args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())), span: None }
    }

    /// 构造带主跨度的解析错误（同时写入 `args.span`）。
    pub fn parse_at(reason: impl Into<std::sync::Arc<str>>, span: SourceSpan) -> Self {
        Self::Parse {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())).with("span", ErrorArg::Span(span)),
            span: Some(span),
        }
    }

    /// 构造无跨度的编译错误（`args.reason`）。
    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile { args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())), span: None }
    }

    /// 解析错误的不透明入口（语义同 [`Self::parse_reason`]）。
    pub fn parse_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::parse_reason(detail)
    }

    /// 编译错误的不透明入口（语义同 [`Self::compile_reason`]）。
    pub fn compile_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::compile_reason(detail)
    }

    /// 主定位跨度；VM 错误恒为 `None`。
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::Parse { span, .. } | Self::Compile { span, .. } => *span,
            Self::Vm(_) => None,
        }
    }

    /// 诊断上下文：目标固定为 `spark-script`，有跨度则附带。
    pub fn context(&self) -> ErrorContext {
        let mut ctx = ErrorContext::new().target("spark-script");
        if let Some(span) = self.span() {
            ctx = ctx.with_span(span);
        }
        ctx
    }

    /// 结构化参数副本；VM 错误委托 [`VmError::args`]。
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
            spark_script_valkyrie::ValkyrieScriptError::Parse { args, span } => ScriptError::Parse { args, span },
            spark_script_valkyrie::ValkyrieScriptError::Compile { args, span } => ScriptError::Compile { args, span },
        }
    }
}

impl From<spark_script_lua::LuaScriptError> for ScriptError {
    fn from(e: spark_script_lua::LuaScriptError) -> Self {
        match e {
            spark_script_lua::LuaScriptError::Parse { args } => ScriptError::Parse { args, span: None },
            spark_script_lua::LuaScriptError::Compile { args } => ScriptError::Compile { args, span: None },
        }
    }
}

impl From<spark_script_ruby::RubyScriptError> for ScriptError {
    fn from(e: spark_script_ruby::RubyScriptError) -> Self {
        match e {
            spark_script_ruby::RubyScriptError::Parse { args } => ScriptError::Parse { args, span: None },
            spark_script_ruby::RubyScriptError::Compile { args } => ScriptError::Compile { args, span: None },
        }
    }
}

/// 仅编译为 [`Module`]（不建 VM；供前端单测）。
pub fn compile_module(language: ScriptLanguage, source: &str, hosts: &HostBindTable) -> Result<Module, ScriptError> {
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
