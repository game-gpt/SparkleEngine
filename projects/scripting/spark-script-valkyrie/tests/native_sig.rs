//! 自 `src/native_sig.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script_valkyrie::*;

#[test]
fn param_carries_name_and_type() {
    let p = NativeParam::new("id", "u32").with_docs("实体编号");
    assert_eq!(p.name.as_ref(), "id");
    assert_eq!(p.ty.as_str(), "u32");
    assert!(p.docs.is_some());
}
