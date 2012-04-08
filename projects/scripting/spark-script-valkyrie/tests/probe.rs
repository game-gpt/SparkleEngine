#[test]
fn probe_valkyrie_sources() {
    use oak_core::{Builder, SourceText};
    use oak_valkyrie::{ValkyrieBuilder, ValkyrieLanguage};
    let language = ValkyrieLanguage {
        allow_legacy_function: true,
        ..ValkyrieLanguage::default()
    };
    let builder = ValkyrieBuilder::new(&language);
    for src in [
        "return 40 + 2;",
        "40 + 2;",
        "let x = 40;",
        "micro f() { return 1; }",
        "micro add(x: i32, y: i32) -> i32 { x + y }",
        "micro PI() { return 3.14 }",
        "namespace Test { micro main() { let x = 42 } }",
        "",
    ] {
        let text = SourceText::new(src);
        let mut session = oak_core::ParseSession::<ValkyrieLanguage>::default();
        let out = builder.build(&text, &[], &mut session);
        eprintln!(
            "SRC={src:?} ok={} items={:?} err={:?}",
            out.result.is_ok(),
            out.result.as_ref().ok().map(|r| r.items.len()),
            out.result.as_ref().err()
        );
    }
}
