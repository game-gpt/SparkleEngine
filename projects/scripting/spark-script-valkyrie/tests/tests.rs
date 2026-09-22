//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_diagnostics::ErrorArg;
use spark_gc::Value;
use spark_ir::HostBindTable;
use spark_script_valkyrie::*;
use spark_vm::{StdHost, Vm};

#[test]
fn arithmetic_main() {
    let m = compile("return 40 + 2").unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn micro_call() {
    let m = compile(
        r#"
        micro add(a, b) {
            return a + b
        }
        return add(40, 2)
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn short_circuit_and_or() {
    // Oaks 当前对 `false && …` / `if true` 解析不稳，用比较表达式覆盖短路。
    let m = compile("return 1 < 0 && 99").unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert!(matches!(v, Value::Bool(false)));

    let m = compile("return 1 < 2 || 0").unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn if_while_via_ir() {
    let m = compile(
        r#"
        if 1 < 2 {
            return 40 + 2
        }
        return 0
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(42.0));

    let m = compile(
        r#"
        while 1 < 0 {
            return 1
        }
        return 42
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn native_register_block() {
    let m = compile_with_binds(
        r#"register_block(1, "astracraft3:dirt", "泥土", "textures/dirt.png", 1, 1, 30, "none")"#,
        &HostBindTable::from_ids([spark_ir::HostId::new("host", "register_block", 1)]).unwrap(),
    )
    .unwrap();
    let mut vm = Vm::new(m);
    vm.prepare_host_slots(["host.register_block"]);
    let called = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let c2 = called.clone();
    vm.register_native("host.register_block", move |_ctx, args: Vec<Value>| {
        assert_eq!(args.len(), 8);
        c2.set(c2.get() + 1);
        Ok(Value::Null)
    });
    let _ = vm.run(&mut StdHost).unwrap();
    assert_eq!(called.get(), 1);
    assert_eq!(vm.call_hits.get("host:0:host.register_block"), Some(&1));
}

#[test]
fn parse_error_carries_source_span() {
    let err = parse("@@@").expect_err("bare attributes");
    assert_eq!(err.to_string(), "spark.script.valkyrie.parse");
    match &err {
        ValkyrieScriptError::Parse { args, span } => {
            assert!(matches!(
                args.get("reason"),
                Some(ErrorArg::String(s)) if s.as_ref() == "parse_failed"
            ));
            assert!(span.is_some());
        }
        _ => panic!("expected Parse"),
    }
}
