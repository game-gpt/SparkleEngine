//! 编译制品层次：目标文件 → 已链接程序 → 可执行映像。
//!
//! [`spark_vm::Module`] 仍是过渡期字节码载体；正式边界要求：
//! - 前端不直接构造最终映像
//! - VM 只接受 [`ExecutableImage`]
//! - 源码模块 / 目标 / 链接结果 / 运行实例分离

use std::sync::Arc;

use spark_vm::{verify_bytecode_with_host, BytecodeVerifyError, Module};

use crate::host_schema::HostSchema;
use crate::request::{LanguageProfile, PackageId};

/// 制品格式版本（与编译器版本独立）。
pub const ARTIFACT_FORMAT_VERSION: u32 = 1;

/// 可重定位目标（单个编译单元，尚未完成跨包链接）。
#[derive(Debug, Clone)]
pub struct SparkObject {
    pub format_version: u32,
    pub compiler_version: Arc<str>,
    pub package: PackageId,
    pub language: LanguageProfile,
    pub host_schema_hash: u64,
    pub host_abi_version: u32,
    /// 过渡期：仍携带前端直接生成的 [`Module`]。后续改为 MIR / 可重定位字节码。
    pub legacy_module: Module,
    pub exports: Vec<Arc<str>>,
    pub imports: Vec<Arc<str>>,
}

impl SparkObject {
    pub fn from_legacy_module(
        package: PackageId,
        language: LanguageProfile,
        host: &HostSchema,
        module: Module,
    ) -> Self {
        let exports = module
            .functions
            .iter()
            .filter(|f| f.name != "__main")
            .map(|f| Arc::<str>::from(f.name.as_str()))
            .collect();
        let imports = module
            .native_names
            .iter()
            .map(|n| Arc::<str>::from(n.as_str()))
            .collect();
        Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            compiler_version: Arc::from(env!("CARGO_PKG_VERSION")),
            package,
            language,
            host_schema_hash: host.content_hash(),
            host_abi_version: host.abi_version,
            legacy_module: module,
            exports,
            imports,
        }
    }
}

/// 已链接程序：稳定函数编号与宿主导入槽位，尚无运行状态。
#[derive(Debug, Clone)]
pub struct LinkedProgram {
    pub format_version: u32,
    pub package: PackageId,
    pub language: LanguageProfile,
    pub host_schema_hash: u64,
    pub host_abi_version: u32,
    /// 链接时宿主函数槽位数（与 [`HostSchema`] 插入顺序一致）。
    pub host_slot_count: u32,
    pub legacy_module: Module,
    /// 导出生命周期名（若存在）。
    pub lifecycle_exports: Vec<Arc<str>>,
}

impl LinkedProgram {
    /// 过渡期单目标「链接」：校验宿主导入、把 `CallNative` 绑成 `CallHost`、记录生命周期导出。
    pub fn link_single(object: SparkObject, host: &HostSchema) -> Result<Self, LinkError> {
        if object.host_abi_version != host.abi_version {
            return Err(LinkError::AbiVersionMismatch {
                object: object.host_abi_version,
                host: host.abi_version,
            });
        }
        if object.host_schema_hash != host.content_hash() {
            return Err(LinkError::HostSchemaMismatch);
        }
        for import in &object.imports {
            if host.get_by_short_name(import).is_none() {
                return Err(LinkError::UnresolvedHost {
                    name: Arc::clone(import),
                });
            }
        }
        let lifecycle_exports = object
            .exports
            .iter()
            .filter(|n| is_lifecycle_export(n))
            .cloned()
            .collect();
        let mut module = object.legacy_module;
        let slot_names = host.short_names();
        spark_vm::bind_host_slots(&mut module, &slot_names).map_err(|detail| {
            if let Some(name) = detail.strip_prefix("unbound_native:") {
                LinkError::UnboundNativeCall {
                    name: Arc::from(name),
                }
            } else {
                LinkError::HostBindFailed {
                    detail: Arc::from(detail),
                }
            }
        })?;
        Ok(Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            package: object.package,
            language: object.language,
            host_schema_hash: object.host_schema_hash,
            host_abi_version: object.host_abi_version,
            host_slot_count: host.functions.len() as u32,
            legacy_module: module,
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
    AbiVersionMismatch { object: u32, host: u32 },
    HostSchemaMismatch,
    UnresolvedHost { name: Arc<str> },
    UnboundNativeCall { name: Arc<str> },
    HostBindFailed { detail: Arc<str> },
}

impl LinkError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AbiVersionMismatch { .. } => "spark.script.link.abi_mismatch",
            Self::HostSchemaMismatch => "spark.script.link.host_schema_mismatch",
            Self::UnresolvedHost { .. } => "spark.script.link.unresolved_host",
            Self::UnboundNativeCall { .. } => "spark.script.link.unbound_native",
            Self::HostBindFailed { .. } => "spark.script.link.host_bind_failed",
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
    Bytecode(BytecodeVerifyError),
}

impl VerifyError {
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
    pub format_version: u32,
    pub package: PackageId,
    pub language: LanguageProfile,
    pub host_schema_hash: u64,
    pub host_abi_version: u32,
    pub host_slot_count: u32,
    pub lifecycle_exports: Vec<Arc<str>>,
    module: Module,
}

impl ExecutableImage {
    /// 对已链接程序做字节码验证后封存（含宿主槽位契约）。
    pub fn verify(program: LinkedProgram) -> Result<Self, VerifyError> {
        verify_bytecode_with_host(&program.legacy_module, program.host_slot_count)?;
        Ok(Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            package: program.package,
            language: program.language,
            host_schema_hash: program.host_schema_hash,
            host_abi_version: program.host_abi_version,
            host_slot_count: program.host_slot_count,
            lifecycle_exports: program.lifecycle_exports,
            module: program.legacy_module,
        })
    }

    /// 装载前再次确认运行期 schema 与编译期一致。
    pub fn check_host_schema(&self, host: &HostSchema) -> Result<(), LinkError> {
        if self.host_abi_version != host.abi_version {
            return Err(LinkError::AbiVersionMismatch {
                object: self.host_abi_version,
                host: host.abi_version,
            });
        }
        if self.host_schema_hash != host.content_hash() {
            return Err(LinkError::HostSchemaMismatch);
        }
        Ok(())
    }

    pub fn module(&self) -> &Module {
        &self.module
    }

    /// 克隆内部模块供过渡期 [`crate::ScriptRuntime`] 装载。
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
        Self {
            format_version,
            package,
            language,
            host_schema_hash,
            host_abi_version,
            host_slot_count,
            lifecycle_exports,
            module,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_schema::{HostFunction, HostFunctionId, HostSchema};
    use spark_script_valkyrie::NativeParam;
    use spark_vm::{FuncProto, Op};

    fn sample_module() -> Module {
        let mut f = FuncProto::new("__main", 0);
        f.emit(Op::LoadNull);
        f.emit(Op::Return);
        Module {
            functions: vec![f],
            entry: 0,
            native_names: vec!["print".into()],
        }
    }

    fn schema_with_print() -> HostSchema {
        let mut schema = HostSchema::new(1);
        schema.insert(
            HostFunction::new(HostFunctionId::new("host", "print", 1))
                .param(NativeParam::new("msg", "String"))
                .returns("Null"),
        );
        schema
    }

    #[test]
    fn link_and_verify_roundtrip() {
        let host = schema_with_print();
        let obj = SparkObject::from_legacy_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            sample_module(),
        );
        let linked = LinkedProgram::link_single(obj, &host).unwrap();
        let image = ExecutableImage::verify(linked).unwrap();
        image.check_host_schema(&host).unwrap();
        assert_eq!(image.module().native_names, vec!["print".to_string()]);
    }

    #[test]
    fn unresolved_host_fails_link() {
        let host = HostSchema::new(1);
        let obj = SparkObject::from_legacy_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &HostSchema::new(1),
            sample_module(),
        );
        let err = LinkedProgram::link_single(obj, &host).unwrap_err();
        assert!(matches!(err, LinkError::UnresolvedHost { .. }));
    }

    #[test]
    fn link_rewrites_call_native_to_call_host() {
        let mut f = FuncProto::new("__main", 0);
        let si = f.add_string("print");
        f.emit(Op::LoadNull);
        f.emit(Op::CallNative);
        f.emit_u16(si);
        f.emit_u8(1);
        f.emit(Op::Return);
        let host = schema_with_print();
        let obj = SparkObject::from_legacy_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            Module {
                functions: vec![f],
                entry: 0,
                native_names: vec!["print".into()],
            },
        );
        let linked = LinkedProgram::link_single(obj, &host).unwrap();
        assert!(linked
            .legacy_module
            .functions[0]
            .code
            .iter()
            .any(|&b| b == Op::CallHost as u8));
        assert!(!linked
            .legacy_module
            .functions[0]
            .code
            .iter()
            .any(|&b| b == Op::CallNative as u8));
    }

    #[test]
    fn unbound_call_native_fails_link() {
        let mut f = FuncProto::new("__main", 0);
        let si = f.add_string("sneaky");
        f.emit(Op::LoadNull);
        f.emit(Op::CallNative);
        f.emit_u16(si);
        f.emit_u8(1);
        f.emit(Op::Return);
        let host = schema_with_print();
        let obj = SparkObject::from_legacy_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            Module {
                functions: vec![f],
                entry: 0,
                // 故意不进 imports：只靠字符串池里的 CallNative
                native_names: Vec::new(),
            },
        );
        let err = LinkedProgram::link_single(obj, &host).unwrap_err();
        assert!(matches!(err, LinkError::UnboundNativeCall { .. }));
    }

    #[test]
    fn verify_rejects_host_slot_oob() {
        let mut f = FuncProto::new("__main", 0);
        f.emit(Op::LoadNull);
        f.emit(Op::CallHost);
        f.emit_u16(5);
        f.emit_u8(1);
        f.emit(Op::Return);
        let program = LinkedProgram {
            format_version: ARTIFACT_FORMAT_VERSION,
            package: PackageId::anonymous(),
            language: crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            host_schema_hash: 0,
            host_abi_version: 1,
            host_slot_count: 1,
            legacy_module: Module {
                functions: vec![f],
                entry: 0,
                native_names: vec!["print".into()],
            },
            lifecycle_exports: Vec::new(),
        };
        let err = ExecutableImage::verify(program).unwrap_err();
        assert!(matches!(
            err,
            VerifyError::Bytecode(BytecodeVerifyError::HostSlotOob { .. })
        ));
    }
}
