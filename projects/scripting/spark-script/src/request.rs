//! 编译请求与语言 profile。
//!
//! 单个 `compile(language, source, natives)` 字符串入口只可作为 REPL / 测试便利，
//! 正式模组编译必须携带 profile、宿主 schema 与能力策略。

use std::{path::PathBuf, sync::Arc};

use crate::{
    ScriptLanguage,
    host_schema::{CapabilityId, DeterminismClass, HostSchema},
};

/// 语言前端选择（与 profile 正交：同一前端可有多个 profile）。
///
/// 决定走哪条 AST → IR 前端实现（`spark-script-valkyrie` / `lua` / `ruby`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageFrontend {
    /// Oaks Valkyrie 前端。
    Valkyrie,
    /// Spark Lua profile 前端（非完整标准 Lua 运行时声明）。
    Lua,
    /// Spark Ruby / RGSS 兼容前端。
    Ruby,
}

impl From<ScriptLanguage> for LanguageFrontend {
    fn from(value: ScriptLanguage) -> Self {
        match value {
            ScriptLanguage::Valkyrie => Self::Valkyrie,
            ScriptLanguage::Lua => Self::Lua,
            ScriptLanguage::Ruby => Self::Ruby,
        }
    }
}

impl From<LanguageFrontend> for ScriptLanguage {
    fn from(value: LanguageFrontend) -> Self {
        match value {
            LanguageFrontend::Valkyrie => Self::Valkyrie,
            LanguageFrontend::Lua => Self::Lua,
            LanguageFrontend::Ruby => Self::Ruby,
        }
    }
}

/// 正式语言语义契约标识。
///
/// 稳定字符串（如 `spark-valkyrie-1`）；写入制品并参与缓存键，升级语义须换 id。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageProfileId {
    /// Profile 稳定标识字符串。
    pub id: Arc<str>,
}

impl LanguageProfileId {
    /// 由任意稳定 id 字符串构造。
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self { id: id.into() }
    }

    /// 返回 profile id 字符串切片。
    pub fn as_str(&self) -> &str {
        &self.id
    }

    /// 默认 Valkyrie 游戏脚本契约：`spark-valkyrie-1`。
    pub fn spark_valkyrie_1() -> Self {
        Self::new("spark-valkyrie-1")
    }

    /// Sparkle Script Edit profile（编辑自动化；绑定 `spark-edit`）。
    pub fn spark_edit_1() -> Self {
        Self::new("spark-edit-1")
    }

    /// Spark Lua 契约：`spark-lua-1`。
    pub fn spark_lua_1() -> Self {
        Self::new("spark-lua-1")
    }

    /// Spark Ruby 契约：`spark-ruby-1`（非 RGSS）。
    pub fn spark_ruby_1() -> Self {
        Self::new("spark-ruby-1")
    }

    /// RGSS 兼容契约：`rgss-compat`（Ruby 前端 + 兼容语义）。
    pub fn rgss_compat() -> Self {
        Self::new("rgss-compat")
    }
}

impl From<&str> for LanguageProfileId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// 语言前端 + profile + 可选语言版本。
///
/// 前端选实现，profile 选语义契约；`language_version` 仅作诊断 / 元数据，不改分派。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageProfile {
    /// 选用的前端实现。
    pub frontend: LanguageFrontend,
    /// 语义契约 id。
    pub profile: LanguageProfileId,
    /// 可选语言版本标注（如 `5.1`）；不参与前端分派。
    pub language_version: Option<Arc<str>>,
}

impl LanguageProfile {
    /// 构造无语言版本标注的 profile。
    pub fn new(frontend: LanguageFrontend, profile: impl Into<LanguageProfileId>) -> Self {
        Self { frontend, profile: profile.into(), language_version: None }
    }

    /// 附带语言版本元数据。
    pub fn with_language_version(mut self, version: impl Into<Arc<str>>) -> Self {
        self.language_version = Some(version.into());
        self
    }

    /// 由 [`ScriptLanguage`] 选择默认 Spark profile（便利映射，非完整语言生态声明）。
    pub fn default_for(language: ScriptLanguage) -> Self {
        match language {
            ScriptLanguage::Valkyrie => Self::new(LanguageFrontend::Valkyrie, LanguageProfileId::spark_valkyrie_1()),
            ScriptLanguage::Lua => Self::new(LanguageFrontend::Lua, LanguageProfileId::spark_lua_1()),
            ScriptLanguage::Ruby => Self::new(LanguageFrontend::Ruby, LanguageProfileId::spark_ruby_1()),
        }
    }
}

/// 包身份（链接与缓存键）。
///
/// `name` + `version` 唯一标识编译单元；匿名包用于 REPL（[`PackageId::anonymous`]）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    /// 包名（模组 / 库标识）。
    pub name: Arc<str>,
    /// 包版本字符串（参与缓存与链接身份，非 semver 强制校验）。
    pub version: Arc<str>,
}

impl PackageId {
    /// 构造具名包身份。
    pub fn new(name: impl Into<Arc<str>>, version: impl Into<Arc<str>>) -> Self {
        Self { name: name.into(), version: version.into() }
    }

    /// REPL / 测试用匿名包：`_anonymous@0`。
    pub fn anonymous() -> Self {
        Self::new("_anonymous", "0")
    }
}

/// 单个源文件输入。
///
/// `path` 用于诊断定位与模组装载；内存源（REPL）可为 `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// 磁盘路径；内存源为 `None`。
    pub path: Option<PathBuf>,
    /// 逻辑模块名（入口列表与诊断用）。
    pub name: Arc<str>,
    /// UTF-8 源文本。
    pub source: Arc<str>,
}

impl SourceFile {
    /// 构造无磁盘路径的内存源（`path = None`）。
    pub fn memory(name: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        Self { path: None, name: name.into(), source: source.into() }
    }
}

/// 优化级别。
///
/// 当前前端可能忽略部分级别；写入请求以便缓存键与未来管线使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizationLevel {
    /// 不做优化（默认；REPL 友好）。
    #[default]
    None,
    /// 基础安全优化（模组默认）。
    Basic,
    /// 激进优化（可能影响调试映射）。
    Aggressive,
}

/// 调试信息级别。
///
/// 控制制品是否携带行表 / 完整符号；默认 [`DebugInfoLevel::LineTables`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DebugInfoLevel {
    /// 不生成调试信息。
    None,
    /// 仅行号表（默认）。
    #[default]
    LineTables,
    /// 完整调试符号。
    Full,
}

/// 正式编译请求。
///
/// 携带包身份、源文件、语言 profile、宿主 schema 与能力 / 确定性策略；
/// [`crate::compiler::ScriptCompiler::compile`] 的权威输入。
#[derive(Debug, Clone)]
pub struct CompilationRequest {
    /// 包身份（链接与缓存键）。
    pub package: PackageId,
    /// 参与本次编译的源文件列表。
    pub sources: Vec<SourceFile>,
    /// 入口模块逻辑名（通常对应 `sources[].name`）。
    pub entry_modules: Vec<Arc<str>>,
    /// 前端 + 语义 profile。
    pub language: LanguageProfile,
    /// 编译与运行须一致的宿主 ABI。
    pub host_schema: HostSchema,
    /// 授予脚本的能力集合（映射为 [`HostCompilePolicy::granted_capabilities`]）。
    pub required_capabilities: Vec<CapabilityId>,
    /// 编译期确定性策略（绑定表不得弱于此）。
    pub determinism: DeterminismClass,
    /// 优化级别。
    pub optimization: OptimizationLevel,
    /// 调试信息级别。
    pub debug_info: DebugInfoLevel,
    /// 可选特性开关令牌（前端按需解释）。
    pub feature_flags: Vec<Arc<str>>,
}

impl CompilationRequest {
    /// REPL / 测试便利：单文件 + 默认 profile。正式模组请用 [`Self::for_mod`]。
    pub fn repl(language: ScriptLanguage, source: impl Into<Arc<str>>, host_schema: HostSchema) -> Self {
        Self {
            package: PackageId::anonymous(),
            sources: vec![SourceFile::memory("<repl>", source)],
            entry_modules: vec![Arc::from("<repl>")],
            language: LanguageProfile::default_for(language),
            host_schema,
            required_capabilities: Vec::new(),
            determinism: DeterminismClass::Nondeterministic,
            optimization: OptimizationLevel::None,
            debug_info: DebugInfoLevel::LineTables,
            feature_flags: Vec::new(),
        }
    }

    /// 正式模组装载：包身份、入口路径与默认 profile（`rgss` → `rgss-compat`）。
    pub fn for_mod(
        package: PackageId,
        language: ScriptLanguage,
        language_token: Option<&str>,
        entry_path: PathBuf,
        entry_name: impl Into<Arc<str>>,
        source: impl Into<Arc<str>>,
        host_schema: HostSchema,
    ) -> Self {
        let entry_name = entry_name.into();
        let language = match language_token.map(|s| s.to_ascii_lowercase()).as_deref() {
            Some("rgss") => LanguageProfile::new(LanguageFrontend::Ruby, LanguageProfileId::rgss_compat()),
            _ => LanguageProfile::default_for(language),
        };
        Self {
            package,
            sources: vec![SourceFile { path: Some(entry_path), name: Arc::clone(&entry_name), source: source.into() }],
            entry_modules: vec![entry_name],
            language,
            host_schema,
            required_capabilities: Vec::new(),
            determinism: DeterminismClass::Nondeterministic,
            optimization: OptimizationLevel::Basic,
            debug_info: DebugInfoLevel::LineTables,
            feature_flags: Vec::new(),
        }
    }

    /// 返回首个源文件的文本；无源时为 `None`（单文件请求的便捷入口）。
    pub fn primary_source(&self) -> Option<&str> {
        self.sources.first().map(|s| s.source.as_ref())
    }
}
