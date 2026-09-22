//! 自 `src/render.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_core::{ErrorArg, SparkError, codes};
use spark_localization::{CompileOptions, LocaleId, LocaleSnapshot, LocalizationDocument, MessageDefinition, compile_documents, render_error};
use std::sync::Arc;

#[test]
fn renders_user_message_from_error_code() {
    let mut doc = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "spark");
    doc.insert("error.asset.not_found", MessageDefinition::Text(Arc::from("Asset not found: {key}")));
    let bundle = compile_documents(&[doc], CompileOptions::default()).unwrap().bundle;
    let snap = LocaleSnapshot::from_bundle(
        LocaleId::parse("en").unwrap(),
        &LocaleId::parse("en").unwrap(),
        &[LocaleId::parse("en").unwrap()],
        1,
        bundle,
    );
    let err = SparkError::new(codes::asset_not_found()).arg("key", ErrorArg::AssetKey(Arc::from("textures/dirt.png")));
    let text = render_error(&snap, &err);
    assert_eq!(text.text.as_ref(), "Asset not found: textures/dirt.png");
    assert!(!text.text.contains("无法"));
}
