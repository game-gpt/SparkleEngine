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
    pub format_version: u32,
    pub compiler_version: Arc<str>,
    pub package: PackageId,
    pub language: LanguageProfile,
    pub host_schema_hash: u64,
    pub host_abi_version: u32,
    /// 本单元字节码（入口函数名必须为 `on_load`）。
    pub module: Module,
    pub exports: Vec<Arc<str>>,
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
    pub format_version: u32,
    pub package: PackageId,
    pub language: LanguageProfile,
    pub host_schema_hash: u64,
    pub host_abi_version: u32,
    /// 链接时宿主函数槽位数（与 [`HostSchema`] 插入顺序一致）。
    pub host_slot_count: u32,
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
    AbiVersionMismatch { object: u32, host: u32 },
    HostSchemaMismatch,
    UnresolvedHost { name: Arc<str> },
    HostBindFailed { detail: Arc<str> },
    EmptyLinkSet,
    MissingEntryPackage { name: Arc<str> },
    MergeFailed { detail: Arc<str> },
    MissingEntryFunction,
    EntryMustBeOnLoad { name: Arc<str> },
    ResidualCallNative,
}

impl LinkError {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_schema::{HostFunction, HostFunctionId, HostSchema};
    use spark_script_valkyrie::NativeParam;
    use spark_vm::{FuncProto, Op};

    fn sample_module() -> Module {
        let mut f = FuncProto::new("on_load", 0);
        f.emit(Op::LoadNull);
        f.emit(Op::Return);
        Module { functions: vec![f], entry: 0, native_names: vec!["print".into()] }
    }

    fn schema_with_print() -> HostSchema {
        let mut schema = HostSchema::new(1);
        schema.insert(HostFunction::new(HostFunctionId::new("host", "print", 1)).param(NativeParam::new("msg", "String")).returns("Null"));
        schema
    }

    #[test]
    fn link_and_verify_roundtrip() {
        let host = schema_with_print();
        let obj = SparkObject::from_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            sample_module(),
        )
        .unwrap();
        let linked = LinkedProgram::link_single(obj, &host).unwrap();
        let image = ExecutableImage::verify(linked).unwrap();
        image.check_host_schema(&host).unwrap();
        assert_eq!(image.module().native_names, vec!["host.print".to_string()]);
    }

    #[test]
    fn unresolved_host_fails_link() {
        let host = HostSchema::new(1);
        let obj = SparkObject::from_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &HostSchema::new(1),
            sample_module(),
        )
        .unwrap();
        let err = LinkedProgram::link_single(obj, &host).unwrap_err();
        assert!(matches!(err, LinkError::UnresolvedHost { .. }));
    }

    #[test]
    fn link_rejects_residual_call_native() {
        let mut f = FuncProto::new("on_load", 0);
        let si = f.add_string("print");
        f.emit(Op::LoadNull);
        f.emit(Op::CallNative);
        f.emit_u16(si);
        f.emit_u8(1);
        f.emit(Op::Return);
        let host = schema_with_print();
        let obj = SparkObject::from_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            Module { functions: vec![f], entry: 0, native_names: vec!["print".into()] },
        )
        .unwrap();
        let err = LinkedProgram::link_single(obj, &host).unwrap_err();
        assert!(matches!(err, LinkError::ResidualCallNative));
    }

    #[test]
    fn link_accepts_call_host() {
        let mut f = FuncProto::new("on_load", 0);
        f.emit(Op::LoadNull);
        f.emit(Op::CallHost);
        f.emit_u16(0);
        f.emit_u8(1);
        f.emit(Op::Return);
        let host = schema_with_print();
        let obj = SparkObject::from_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            Module { functions: vec![f], entry: 0, native_names: vec!["print".into()] },
        )
        .unwrap();
        let linked = LinkedProgram::link_single(obj, &host).unwrap();
        assert!(linked.module.functions[0].code.iter().any(|&b| b == Op::CallHost as u8));
        assert!(!linked.module.functions[0].code.iter().any(|&b| b == Op::CallNative as u8));
    }

    #[test]
    fn residual_call_native_outside_imports_fails_link() {
        let mut f = FuncProto::new("on_load", 0);
        let si = f.add_string("sneaky");
        f.emit(Op::LoadNull);
        f.emit(Op::CallNative);
        f.emit_u16(si);
        f.emit_u8(1);
        f.emit(Op::Return);
        let host = schema_with_print();
        let obj = SparkObject::from_module(
            PackageId::anonymous(),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            Module {
                functions: vec![f],
                entry: 0,
                // 故意不进 imports：残留 CallNative 仍须在链接期拒绝
                native_names: Vec::new(),
            },
        )
        .unwrap();
        let err = LinkedProgram::link_single(obj, &host).unwrap_err();
        assert!(matches!(err, LinkError::ResidualCallNative));
    }

    #[test]
    fn verify_rejects_host_slot_oob() {
        let mut f = FuncProto::new("on_load", 0);
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
            module: Module { functions: vec![f], entry: 0, native_names: vec!["print".into()] },
            lifecycle_exports: Vec::new(),
        };
        let err = ExecutableImage::verify(program).unwrap_err();
        assert!(matches!(err, VerifyError::Bytecode(BytecodeVerifyError::HostSlotOob { .. })));
    }

    #[test]
    fn link_many_merges_library_into_entry() {
        let host = schema_with_print();

        let mut lib_fn = FuncProto::new("double", 1);
        lib_fn.locals = 1;
        lib_fn.emit(Op::LoadLocal);
        lib_fn.emit_u16(0);
        lib_fn.emit(Op::LoadLocal);
        lib_fn.emit_u16(0);
        lib_fn.emit(Op::Add);
        lib_fn.emit(Op::Return);
        let mut lib_main = FuncProto::new("on_load", 0);
        lib_main.emit(Op::LoadNull);
        lib_main.emit(Op::Return);
        let lib_obj = SparkObject::from_module(
            PackageId::new("lib", "1"),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            Module { functions: vec![lib_fn, lib_main], entry: 1, native_names: Vec::new() },
        )
        .unwrap();

        let mut entry_main = FuncProto::new("on_load", 0);
        let twenty_one = entry_main.add_const_number(21.0);
        // double 在入口模块下标 0；链接后按名重映射。
        let double_ref = entry_main.add_const_func(0);
        entry_main.emit(Op::LoadConst);
        entry_main.emit_u16(double_ref);
        entry_main.emit(Op::LoadConst);
        entry_main.emit_u16(twenty_one);
        entry_main.emit(Op::Call);
        entry_main.emit_u8(1);
        entry_main.emit(Op::Return);
        // 入口模块也声明同名 stub，供本模块内 Func 下标解析；链接时以先出现的库函数为准。
        let mut stub = FuncProto::new("double", 1);
        stub.locals = 1;
        stub.emit(Op::LoadLocal);
        stub.emit_u16(0);
        stub.emit(Op::Return);
        let entry_obj = SparkObject::from_module(
            PackageId::new("app", "1"),
            crate::request::LanguageProfile::default_for(crate::ScriptLanguage::Valkyrie),
            &host,
            Module { functions: vec![stub, entry_main], entry: 1, native_names: Vec::new() },
        )
        .unwrap();

        // 显式入口包 `app`。
        let entry_pkg = PackageId::new("app", "1");
        let linked = LinkedProgram::link_many(&[lib_obj, entry_obj], &host, &entry_pkg).unwrap();
        let image = ExecutableImage::verify(linked).unwrap();
        assert!(image.module().functions.iter().any(|f| f.name == "double"));
        assert!(image.module().functions.iter().any(|f| f.name == "on_load"));
        let mut vm = spark_vm::Vm::new(image.clone_module());
        let v = vm.run(&mut spark_vm::StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn link_many_empty_fails() {
        let host = HostSchema::new(1);
        let err = LinkedProgram::link_many(&[], &host, &PackageId::anonymous()).unwrap_err();
        assert!(matches!(err, LinkError::EmptyLinkSet));
    }
}
