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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageFrontend {
    Valkyrie,
    Lua,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageProfileId {
    pub id: Arc<str>,
}

impl LanguageProfileId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self { id: id.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.id
    }

    pub fn spark_valkyrie_1() -> Self {
        Self::new("spark-valkyrie-1")
    }

    pub fn spark_lua_1() -> Self {
        Self::new("spark-lua-1")
    }

    pub fn spark_ruby_1() -> Self {
        Self::new("spark-ruby-1")
    }

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageProfile {
    pub frontend: LanguageFrontend,
    pub profile: LanguageProfileId,
    pub language_version: Option<Arc<str>>,
}

impl LanguageProfile {
    pub fn new(frontend: LanguageFrontend, profile: impl Into<LanguageProfileId>) -> Self {
        Self { frontend, profile: profile.into(), language_version: None }
    }

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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    pub name: Arc<str>,
    pub version: Arc<str>,
}

impl PackageId {
    pub fn new(name: impl Into<Arc<str>>, version: impl Into<Arc<str>>) -> Self {
        Self { name: name.into(), version: version.into() }
    }

    pub fn anonymous() -> Self {
        Self::new("_anonymous", "0")
    }
}

/// 单个源文件输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: Option<PathBuf>,
    pub name: Arc<str>,
    pub source: Arc<str>,
}

impl SourceFile {
    pub fn memory(name: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        Self { path: None, name: name.into(), source: source.into() }
    }
}

/// 优化级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizationLevel {
    #[default]
    None,
    Basic,
    Aggressive,
}

/// 调试信息级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DebugInfoLevel {
    None,
    #[default]
    LineTables,
    Full,
}

/// 正式编译请求。
#[derive(Debug, Clone)]
pub struct CompilationRequest {
    pub package: PackageId,
    pub sources: Vec<SourceFile>,
    pub entry_modules: Vec<Arc<str>>,
    pub language: LanguageProfile,
    pub host_schema: HostSchema,
    pub required_capabilities: Vec<CapabilityId>,
    pub determinism: DeterminismClass,
    pub optimization: OptimizationLevel,
    pub debug_info: DebugInfoLevel,
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

    pub fn primary_source(&self) -> Option<&str> {
        self.sources.first().map(|s| s.source.as_ref())
    }
}
