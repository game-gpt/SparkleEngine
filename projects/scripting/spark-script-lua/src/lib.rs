//! Oaks `oak-lua` 解析 → 公共 IR → `spark-vm` 字节码。
//!
//! 面向游戏脚本的 Lua 5.x **子集**（函数 / local / 控制流 / 算术）。
//! 完整语义（table、元表、协程）不在本前端范围。
//! 正式编译只经 `spark-ir`；不支持的构造必须报错。

#![forbid(missing_docs)]
mod lower;

use lower::lower_root_to_hir;

use oak_core::{Builder, SourceText};
use oak_lua::{LuaBuilder, LuaLanguage, LuaRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs};
use spark_ir::{HostBindTable, HostEmitMode, emit_module_with_host, lower_module};
use spark_vm::Module;

/// Lua 前端编译失败。
///
/// `Display` 只输出稳定错误码（`spark.script.lua.*`），自然语言细节放在 `args` 里供诊断管线消费。
#[derive(Debug)]
pub enum LuaScriptError {
    /// Oaks 解析失败。
    Parse {
        /// 结构化参数（如 `diagnostics` 计数、`reason` 令牌）。
        args: ErrorArgs,
    },
    /// 降到 IR / 发射字节码失败。
    Compile {
        /// 结构化参数（`reason` 等机器令牌，非 Locale 句子）。
        args: ErrorArgs,
    },
}

impl LuaScriptError {
    /// 构造解析失败：附带 Oaks 诊断条数。
    pub fn parse_failed(diag_count: u64) -> Self {
        Self::Parse {
            args: ErrorArgs::new()
                .with("reason", ErrorArg::String(std::sync::Arc::from("parse_failed")))
                .with("diagnostics", ErrorArg::Unsigned(diag_count)),
        }
    }

    /// 构造编译失败：`reason` 为机器可读令牌。
    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile { args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())) }
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
pub fn compile_with_binds(source: &str, hosts: &HostBindTable) -> Result<Module, LuaScriptError> {
    let root = parse(source)?;
    let hir = lower_root_to_hir(&root, hosts).map_err(LuaScriptError::compile_opaque)?;
    let mir = lower_module(&hir).map_err(LuaScriptError::compile_opaque)?;
    let mode = if hosts.is_empty() { HostEmitMode::NoHost } else { HostEmitMode::Bound(hosts) };
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
