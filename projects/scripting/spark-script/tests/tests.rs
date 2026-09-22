//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script::*;

use spark_vm::StdHost;

fn eval_source(language: ScriptLanguage, source: &str) -> spark_gc::Value {
    let host = HostSchema::new(1);
    let package = ScriptCompiler::new().compile_source(language, source, &host).unwrap();
    let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
    rt.call_on_load_std().unwrap()
}

#[test]
fn valkyrie_arithmetic() {
    let v = eval_source(ScriptLanguage::Valkyrie, "return 40 + 2");
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn lua_function() {
    let v = eval_source(
        ScriptLanguage::Lua,
        r#"
        function add(a, b)
          return a + b
        end
        return add(40, 2)
        "#,
    );
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn ruby_method() {
    let v = eval_source(ScriptLanguage::Ruby, "def add(a, b)\n  return a + b\nend\nreturn add(40, 2)\n");
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn call_micro_from_host() {
    let host = HostSchema::new(1);
    let package = ScriptCompiler::new()
        .compile_source(
            ScriptLanguage::Valkyrie,
            r#"
            micro add(a, b) {
                return a + b
            }
            return 0
            "#,
            &host,
        )
        .unwrap();
    let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
    let v = rt.call("add", &[spark_gc::Value::Number(40.0), spark_gc::Value::Number(2.0)], &mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn host_schema_compile_and_call() {
    let mut host = HostSchema::new(1);
    host.insert(HostFunction::new(HostFunctionId::new("host", "ping", 1)));
    let package = ScriptCompiler::new().compile_source(ScriptLanguage::Valkyrie, "return ping()", &host).unwrap();
    let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
    rt.vm.register_native("host.ping", |_ctx, _args| Ok(spark_gc::Value::Number(7.0)));
    let v = rt.call_on_load_std().unwrap();
    assert_eq!(v.as_number(), Some(7.0));
}

#[test]
fn lua_and_ruby_host_schema_compile() {
    let mut host = HostSchema::new(1);
    host.insert(HostFunction::new(HostFunctionId::new("host", "ping", 1)));
    for lang in [ScriptLanguage::Lua, ScriptLanguage::Ruby] {
        let source = match lang {
            ScriptLanguage::Lua => "return ping(1)",
            ScriptLanguage::Ruby => "return ping(1)",
            ScriptLanguage::Valkyrie => unreachable!(),
        };
        let package = ScriptCompiler::new().compile_source(lang, source, &host).unwrap();
        let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
        rt.vm.register_native("host.ping", |_ctx, args| Ok(args.first().cloned().unwrap_or(spark_gc::Value::Null)));
        let v = rt.call_on_load_std().unwrap();
        assert_eq!(v.as_number(), Some(1.0));
    }
}

#[test]
fn valkyrie_parse_error_propagates_span() {
    let err = compile_module(ScriptLanguage::Valkyrie, "@@@", &HostBindTable::new()).expect_err("bare attributes");
    assert_eq!(err.code(), "spark.script.parse");
}

#[test]
fn compiler_produces_executable_image() {
    let host = HostSchema::new(1);
    let package = ScriptCompiler::new().compile_source(ScriptLanguage::Valkyrie, "return 1 + 2", &host).unwrap();
    assert!(package.image.module().functions.iter().any(|f| f.name == "on_load"));
    let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
    assert_eq!(rt.call_on_load_std().unwrap().as_number(), Some(3.0));
}

#[test]
fn runtime_host_slot_call_executes_registered_native() {
    let mut host = HostSchema::new(1);
    host.insert(HostFunction::new(HostFunctionId::new("host", "ping", 1)));
    let package = ScriptCompiler::new().compile_source(ScriptLanguage::Valkyrie, "return ping()", &host).unwrap();
    let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
    rt.vm.register_native("host.ping", |_ctx, _args| Ok(spark_gc::Value::Number(9.0)));
    assert_eq!(rt.call_on_load_std().unwrap().as_number(), Some(9.0));
}
