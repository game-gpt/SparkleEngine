fn main() {
    use oak_core::{Builder, SourceText};
    use oak_valkyrie::{ValkyrieBuilder, ValkyrieLanguage};
    let language = ValkyrieLanguage { allow_legacy_function: true, ..ValkyrieLanguage::default() };
    let builder = ValkyrieBuilder::new(&language);
    for src in ["return 40 + 2", "40 + 2", "let x = 40", "micro f() { return 1 }", "micro f() { 1 }"] {
        let text = SourceText::new(src);
        let mut session = oak_core::ParseSession::<ValkyrieLanguage>::default();
        let out = builder.build(&text, &[], &mut session);
        println!("SRC={src:?} ok={} diag={:?}", out.result.is_ok(), out.diagnostics.len());
        if let Err(e) = &out.result { println!("  err={e}"); }
        if let Ok(root) = &out.result { println!("  items={}", root.items.len()); }
    }
}
