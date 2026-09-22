//! 自 `src/spkx.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script::*;

use spark_script::{
    LinkedProgram, ScriptLanguage, SparkObject,
    codec::SPKX_MAGIC,
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

fn sample_image() -> ExecutableImage {
    let mut f = FuncProto::new("on_load", 0);
    let c = f.add_const_number(42.0);
    f.emit(Op::LoadConst);
    f.emit_u16(c);
    f.emit(Op::Return);
    let host = schema_with_print();
    let obj = SparkObject::from_module(
        PackageId::new("demo", "1"),
        LanguageProfile::default_for(ScriptLanguage::Valkyrie),
        &host,
        Module { functions: vec![f], entry: 0, native_names: vec!["print".into()] },
    )
    .unwrap();
    let linked = LinkedProgram::link_single(obj, &host).unwrap();
    ExecutableImage::verify(linked).unwrap()
}

#[test]
fn spkx_roundtrip_preserves_module() {
    let image = sample_image();
    let bytes = image.to_spkx_bytes().unwrap();
    assert_eq!(&bytes[..4], SPKX_MAGIC);
    let loaded = ExecutableImage::from_spkx_bytes(&bytes).unwrap();
    assert_eq!(loaded.host_schema_hash, image.host_schema_hash);
    assert_eq!(loaded.host_slot_count, image.host_slot_count);
    assert_eq!(loaded.package.name.as_ref(), "demo");
    assert_eq!(loaded.module().entry, image.module().entry);
    assert_eq!(loaded.module().functions[0].code, image.module().functions[0].code);
    assert_eq!(loaded.module().functions[0].consts[0].as_number(), Some(42.0));
}

#[test]
fn bad_magic_rejected() {
    let err = ExecutableImage::from_spkx_bytes(b"XXXX").unwrap_err();
    assert!(matches!(err, ArtifactIoError::BadMagic));
}
