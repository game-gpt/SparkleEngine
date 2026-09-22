//! 编译制品层次：目标文件 → 已链接程序 → 可执行映像。
//!
//! 字节码载体仍是 [`spark_vm::Module`]（实现细节，不是公共语义入口）。
//! 正式边界：
//! - 前端经 IR 产出模块，再封为 [`SparkObject`]
//! - 运行只装载 [`ExecutableImage`]
//! - 模组入口是生命周期导出（`on_load` 等）

use std::sync::Arc;

use spark_vm::{BytecodeVerifyError, Module, verify_bytecode_with_host};

use crate::{
    host_schema::HostSchema,
    request::{LanguageProfile, PackageId},
};

/// 制品格式版本（与编译器版本独立）。
pub const ARTIFACT_FORMAT_VERSION: u32 = 1;

/// 可重定位目标（单个编译单元，尚未完成跨包链接）。
#[derive(Debug, Clone)]
pub struct SparkObject {
    /// 磁盘 / 内存制品格式版本，须等于 [`ARTIFACT_FORMAT_VERSION`]。
    pub format_version: u32,
    /// 产出本目标的 `spark-script` 包版本字符串。
    pub compiler_version: Arc<str>,
    /// 本单元所属包身份（链接与缓存键）。
    pub package: PackageId,
    /// 编译时语言前端 + profile 契约。
    pub language: LanguageProfile,
    /// 编译期 [`HostSchema::content_hash`]；装载须与运行期 schema 一致。
    pub host_schema_hash: u64,
    /// 编译期宿主 ABI 版本；与 schema 哈希一并校验。
    pub host_abi_version: u32,
    /// 本单元字节码（入口函数名必须为 `on_load`）。
    pub module: Module,
    /// 本单元导出的函数名（含生命周期钩子）。
    pub exports: Vec<Arc<str>>,
    /// 尚未解析的宿主导入名（短名或限定名）。
    pub imports: Vec<Arc<str>>,
}

impl SparkObject {
    /// 由前端字节码封成目标。入口函数名必须为 `on_load`。
    pub fn from_module(package: PackageId, language: LanguageProfile, host: &HostSchema, module: Module) -> Result<Self, LinkError> {
        let entry = module.functions.get(module.entry).ok_or(LinkError::MissingEntryFunction)?;
        if entry.name != "on_load" {
            return Err(LinkError::EntryMustBeOnLoad { name: Arc::from(entry.name.as_str()) });
        }
        let exports = module.functions.iter().map(|f| Arc::<str>::from(f.name.as_str())).collect();
        let imports = module.native_names.iter().map(|n| Arc::<str>::from(n.as_str())).collect();
        Ok(Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            compiler_version: Arc::from(env!("CARGO_PKG_VERSION")),
            package,
            language,
            host_schema_hash: host.content_hash(),
            host_abi_version: host.abi_version,
            module,
            exports,
            imports,
        })
    }
}

/// 已链接程序：稳定函数编号与宿主导入槽位，尚无运行状态。
#[derive(Debug, Clone)]
pub struct LinkedProgram {
    /// 制品格式版本。
    pub format_version: u32,
    /// 入口包身份（多目标链接时取 `entry_package`）。
    pub package: PackageId,
    /// 入口包语言契约。
    pub language: LanguageProfile,
    /// 链接时锁定的宿主 schema 指纹。
    pub host_schema_hash: u64,
    /// 链接时锁定的宿主 ABI 版本。
    pub host_abi_version: u32,
    /// 链接时宿主函数槽位数（与 [`HostSchema`] 插入顺序一致）。
    pub host_slot_count: u32,
    /// 合并后的字节码模块；`native_names` 已替换为 schema 限定名。
    pub module: Module,
    /// 导出生命周期名（若存在）。
    pub lifecycle_exports: Vec<Arc<str>>,
}

impl LinkedProgram {
    /// 单目标链接：校验宿主导入、拒绝残留 `CallNative`、记录生命周期导出。
    pub fn link_single(object: SparkObject, host: &HostSchema) -> Result<Self, LinkError> {
        if object.host_abi_version != host.abi_version {
            return Err(LinkError::AbiVersionMismatch { object: object.host_abi_version, host: host.abi_version });
        }
        if object.host_schema_hash != host.content_hash() {
            return Err(LinkError::HostSchemaMismatch);
        }
        for import in &object.imports {
            if host.resolve_import(import).is_err() {
                return Err(LinkError::UnresolvedHost { name: Arc::clone(import) });
            }
        }
        let lifecycle_exports = object.exports.iter().filter(|n| is_lifecycle_export(n)).cloned().collect();
        let mut module = object.module;
        spark_vm::reject_residual_call_native(&module).map_err(|detail| {
            if detail.starts_with("residual_call_native") {
                LinkError::ResidualCallNative
            }
            else {
                LinkError::HostBindFailed { detail: Arc::from(detail) }
            }
        })?;
        module.native_names = host.qualified_names();
        Ok(Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            package: object.package,
            language: object.language,
            host_schema_hash: object.host_schema_hash,
            host_abi_version: object.host_abi_version,
            host_slot_count: host.functions.len() as u32,
            module,
            lifecycle_exports,
        })
    }

    /// 多目标链接：须显式指定入口包（不再默认 `objects[0]`）。
    pub fn link_many(objects: &[SparkObject], host: &HostSchema, entry_package: &PackageId) -> Result<Self, LinkError> {
        if objects.is_empty() {
            return Err(LinkError::EmptyLinkSet);
        }
        let entry_index = objects
            .iter()
            .position(|o| o.package.name == entry_package.name && o.package.version == entry_package.version)
            .ok_or_else(|| LinkError::MissingEntryPackage { name: Arc::clone(&entry_package.name) })?;
        for object in objects {
            if object.host_abi_version != host.abi_version {
                return Err(LinkError::AbiVersionMismatch { object: object.host_abi_version, host: host.abi_version });
            }
            if object.host_schema_hash != host.content_hash() {
                return Err(LinkError::HostSchemaMismatch);
            }
            for import in &object.imports {
                if host.resolve_import(import).is_err() {
                    return Err(LinkError::UnresolvedHost { name: Arc::clone(import) });
                }
            }
        }
        let mut lifecycle_exports = Vec::new();
        for object in objects {
            for n in &object.exports {
                if is_lifecycle_export(n) && !lifecycle_exports.iter().any(|e: &Arc<str>| e.as_ref() == n.as_ref()) {
                    lifecycle_exports.push(Arc::clone(n));
                }
            }
        }
        let modules: Vec<Module> = objects.iter().map(|o| o.module.clone()).collect();
        let mut module =
            Module::link_with_entry(&modules, entry_index).map_err(|detail| LinkError::MergeFailed { detail: Arc::from(detail) })?;
        spark_vm::reject_residual_call_native(&module).map_err(|detail| {
            if detail.starts_with("residual_call_native") {
                LinkError::ResidualCallNative
            }
            else {
                LinkError::HostBindFailed { detail: Arc::from(detail) }
            }
        })?;
        module.native_names = host.qualified_names();
        let primary = &objects[entry_index];
        Ok(Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            package: primary.package.clone(),
            language: primary.language.clone(),
            host_schema_hash: primary.host_schema_hash,
            host_abi_version: primary.host_abi_version,
            host_slot_count: host.functions.len() as u32,
            module,
            lifecycle_exports,
        })
    }
}

fn is_lifecycle_export(name: &str) -> bool {
    matches!(
        name,
        "on_load"
            | "on_start"
            | "fixed_update"
            | "update"
            | "late_update"
            | "render_prepare"
            | "on_event"
            | "on_unload"
            | "save_state"
            | "load_state"
    )
}

/// 链接错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    /// 目标与运行 / 链接用 schema 的 ABI 版本不一致。
    ///
    /// 稳定码：`spark.script.link.abi_mismatch`。
    AbiVersionMismatch {
        /// 制品内记录的 ABI 版本。
        object: u32,
        /// 当前 [`HostSchema::abi_version`]。
        host: u32,
    },
    /// 宿主 schema 内容指纹不一致。
    ///
    /// 稳定码：`spark.script.link.host_schema_mismatch`。
    HostSchemaMismatch,
    /// 导入名无法在 schema 中解析（未知或短名冲突）。
    ///
    /// 稳定码：`spark.script.link.unresolved_host`。
    UnresolvedHost {
        /// 失败的导入字符串。
        name: Arc<str>,
    },
    /// 宿主绑定后处理失败（非残留 `CallNative` 的其它校验细节）。
    ///
    /// 稳定码：`spark.script.link.host_bind_failed`。
    HostBindFailed {
        /// 来自 `spark-vm` 的细节令牌。
        detail: Arc<str>,
    },
    /// 多目标链接集合为空。
    ///
    /// 稳定码：`spark.script.link.empty_set`。
    EmptyLinkSet,
    /// `entry_package` 不在链接集合中。
    ///
    /// 稳定码：`spark.script.link.missing_entry_package`。
    MissingEntryPackage {
        /// 缺失入口包的名字（不含版本）。
        name: Arc<str>,
    },
    /// `Module::link_with_entry` 合并失败。
    ///
    /// 稳定码：`spark.script.link.merge_failed`。
    MergeFailed {
        /// 合并器返回的细节。
        detail: Arc<str>,
    },
    /// 模块缺少入口函数下标。
    ///
    /// 稳定码：`spark.script.link.missing_entry_function`。
    MissingEntryFunction,
    /// 入口函数名不是强制的 `on_load`。
    ///
    /// 稳定码：`spark.script.link.entry_must_be_on_load`。
    EntryMustBeOnLoad {
        /// 实际入口函数名。
        name: Arc<str>,
    },
    /// 字节码仍含未降低的 `CallNative`。
    ///
    /// 稳定码：`spark.script.link.residual_call_native`。
    ResidualCallNative,
}

impl LinkError {
    /// 稳定错误码。
    pub fn code(&self) -> &'static str {
        match self {
            Self::AbiVersionMismatch { .. } => "spark.script.link.abi_mismatch",
            Self::HostSchemaMismatch => "spark.script.link.host_schema_mismatch",
            Self::UnresolvedHost { .. } => "spark.script.link.unresolved_host",
            Self::HostBindFailed { .. } => "spark.script.link.host_bind_failed",
            Self::EmptyLinkSet => "spark.script.link.empty_set",
            Self::MissingEntryPackage { .. } => "spark.script.link.missing_entry_package",
            Self::MergeFailed { .. } => "spark.script.link.merge_failed",
            Self::MissingEntryFunction => "spark.script.link.missing_entry_function",
            Self::EntryMustBeOnLoad { .. } => "spark.script.link.entry_must_be_on_load",
            Self::ResidualCallNative => "spark.script.link.residual_call_native",
        }
    }
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for LinkError {}

/// 字节码验证错误（包装 `spark-vm` 验证器）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// 委托 [`BytecodeVerifyError`]；稳定码透传其 `code()`。
    Bytecode(BytecodeVerifyError),
}

impl VerifyError {
    /// 稳定错误码（透传底层验证器）。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Bytecode(e) => e.code(),
        }
    }
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for VerifyError {}

impl From<BytecodeVerifyError> for VerifyError {
    fn from(value: BytecodeVerifyError) -> Self {
        Self::Bytecode(value)
    }
}

/// 经过验证、可供 VM 装载的不可变映像。
#[derive(Debug, Clone)]
pub struct ExecutableImage {
    /// 制品格式版本。
    pub format_version: u32,
    /// 入口包身份。
    pub package: PackageId,
    /// 入口包语言契约。
    pub language: LanguageProfile,
    /// 验证时锁定的宿主 schema 指纹。
    pub host_schema_hash: u64,
    /// 验证时锁定的宿主 ABI 版本。
    pub host_abi_version: u32,
    /// 验证时声明的宿主槽位数（解码 `.spkx` 时再次校验）。
    pub host_slot_count: u32,
    /// 已知生命周期导出名。
    pub lifecycle_exports: Vec<Arc<str>>,
    module: Module,
}

impl ExecutableImage {
    /// 对已链接程序做字节码验证后封存（含宿主槽位契约）。
    pub fn verify(program: LinkedProgram) -> Result<Self, VerifyError> {
        verify_bytecode_with_host(&program.module, program.host_slot_count)?;
        Ok(Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            package: program.package,
            language: program.language,
            host_schema_hash: program.host_schema_hash,
            host_abi_version: program.host_abi_version,
            host_slot_count: program.host_slot_count,
            lifecycle_exports: program.lifecycle_exports,
            module: program.module,
        })
    }

    /// 装载前再次确认运行期 schema 与编译期一致。
    pub fn check_host_schema(&self, host: &HostSchema) -> Result<(), LinkError> {
        if self.host_abi_version != host.abi_version {
            return Err(LinkError::AbiVersionMismatch { object: self.host_abi_version, host: host.abi_version });
        }
        if self.host_schema_hash != host.content_hash() {
            return Err(LinkError::HostSchemaMismatch);
        }
        Ok(())
    }

    /// 只读访问内部 [`Module`]（不转移所有权）。
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// 克隆内部模块供 [`crate::ScriptRuntime`] 装载。
    pub fn clone_module(&self) -> Module {
        self.module.clone()
    }

    /// 由解码器组装映像（调用方须已完成字节码验证）。
    pub(crate) fn from_decoded(
        format_version: u32,
        package: PackageId,
        language: LanguageProfile,
        host_schema_hash: u64,
        host_abi_version: u32,
        host_slot_count: u32,
        lifecycle_exports: Vec<Arc<str>>,
        module: Module,
    ) -> Self {
        Self { format_version, package, language, host_schema_hash, host_abi_version, host_slot_count, lifecycle_exports, module }
    }
}
