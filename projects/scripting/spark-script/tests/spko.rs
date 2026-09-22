//! 自 `src/spko.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script::*;
use spark_vm::StdHost;

use spark_script::{
    ExecutableImage, LinkedProgram, ScriptLanguage,
    codec::SPKO_MAGIC,
    host_schema::{HostFunction, HostFunctionId, HostSchema},
    request::{LanguageProfile, PackageId},
};
use spark_script_valkyrie::NativeParam;
use spark_vm::{FuncProto, Module, Op};

fn schema_with_print() -> HostSchema {
    let mut schema = HostSchema::new(1);
    schema.insert(HostFunction::new(HostFunctionId::new("host", "print", 1)).param(NativeParam::new("msg", "String")).returns("Null"));
    schema
}

#[test]
fn spko_roundtrip_then_link() {
    let mut f = FuncProto::new("on_load", 0);
    let c = f.add_const_number(7.0);
    f.emit(Op::LoadConst);
    f.emit_u16(c);
    f.emit(Op::Return);
    let host = schema_with_print();
    let obj = SparkObject::from_module(
        PackageId::new("unit", "0.1"),
        LanguageProfile::default_for(ScriptLanguage::Valkyrie),
        &host,
        Module { functions: vec![f], entry: 0, native_names: vec!["print".into()] },
    )
    .unwrap();
    let bytes = obj.to_spko_bytes().unwrap();
    assert_eq!(&bytes[..4], SPKO_MAGIC);
    let loaded = SparkObject::from_spko_bytes(&bytes).unwrap();
    assert_eq!(loaded.package.name.as_ref(), "unit");
    assert_eq!(loaded.imports.len(), 1);
    let linked = LinkedProgram::link_single(loaded, &host).unwrap();
    let image = ExecutableImage::verify(linked).unwrap();
    let mut vm = spark_vm::Vm::new(image.clone_module());
    let v = vm.run(&mut spark_vm::StdHost).unwrap();
    assert_eq!(v.as_number(), Some(7.0));
}

#[test]
fn spko_bad_magic() {
    let err = SparkObject::from_spko_bytes(b"NOPE").unwrap_err();
    assert!(matches!(err, ArtifactIoError::BadMagic));
}
