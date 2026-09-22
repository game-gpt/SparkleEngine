//! 自 `src/artifact.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script::*;
use spark_vm::StdHost;

use spark_script::{HostFunction, HostFunctionId, HostSchema};
use spark_script_valkyrie::NativeParam;
use spark_vm::{FuncProto, Module, Op};

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
        spark_script::LanguageProfile::default_for(spark_script::ScriptLanguage::Valkyrie),
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
        spark_script::LanguageProfile::default_for(spark_script::ScriptLanguage::Valkyrie),
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
        spark_script::LanguageProfile::default_for(spark_script::ScriptLanguage::Valkyrie),
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
        spark_script::LanguageProfile::default_for(spark_script::ScriptLanguage::Valkyrie),
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
        spark_script::LanguageProfile::default_for(spark_script::ScriptLanguage::Valkyrie),
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
        language: spark_script::LanguageProfile::default_for(spark_script::ScriptLanguage::Valkyrie),
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
        spark_script::LanguageProfile::default_for(spark_script::ScriptLanguage::Valkyrie),
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
        spark_script::LanguageProfile::default_for(spark_script::ScriptLanguage::Valkyrie),
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
