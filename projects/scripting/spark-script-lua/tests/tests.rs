//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_gc::Value;
use spark_ir::HostBindTable;
use spark_script_lua::*;
use spark_vm::{Op, StdHost, Vm};

#[test]
fn unsupported_table_literal_fails() {
    // table 构造不在 IR 子集；必须明确失败，必须明确失败。
    let err = compile("return {a=1}").unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("ir_unsupported") || msg.contains("unsupported") || msg.contains("reason"), "{msg}");
}

#[test]
fn arithmetic_main() {
    let m = compile("return 40 + 2").unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn arithmetic_via_ir_has_no_call_native() {
    let m = compile("return 40 + 2").unwrap();
    assert!(m.functions.iter().all(|f| { !f.code.iter().any(|&b| b == Op::CallNative as u8 || b == Op::CallHost as u8) }));
}

#[test]
fn local_and_if_via_ir() {
    let m = compile(
        r#"
        local x = 1
        if x < 2 then
            return 42
        else
            return 0
        end
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn elseif_and_do_via_ir() {
    let m = compile(
        r#"
        local x = 2
        if x < 1 then
            return 0
        elseif x < 3 then
            return 42
        else
            return 1
        end
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    assert_eq!(vm.run(&mut StdHost).unwrap().as_number(), Some(42.0));

    let m = compile(
        r#"
        local n = 0
        do
            n = n + 40
            n = n + 2
        end
        return n
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    assert_eq!(vm.run(&mut StdHost).unwrap().as_number(), Some(42.0));
}

#[test]
fn repeat_until_via_ir() {
    let m = compile(
        r#"
        local n = 0
        repeat
            n = n + 1
        until n >= 3
        return n
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    assert_eq!(vm.run(&mut StdHost).unwrap().as_number(), Some(3.0));
}

#[test]
fn and_or_via_ir() {
    let m = compile(
        r#"
        local a = 0
        local b = 7
        if a and b then
            return 1
        end
        if a or b then
            return 42
        end
        return 0
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    assert_eq!(vm.run(&mut StdHost).unwrap().as_number(), Some(42.0));
}

#[test]
fn function_call() {
    let m = compile(
        r#"
        function double(n)
            return n * 2
        end
        return double(21)
        "#,
    )
    .unwrap();
    assert!(m.functions.iter().any(|f| f.name == "double"), "expected IR function proto");
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn while_via_ir() {
    let m = compile(
        r#"
        local n = 0
        while n < 3 do
            n = n + 1
        end
        return n
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(m);
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(3.0));
}

#[test]
fn host_call_via_ir() {
    let m = compile_with_binds("return ping(7)", &HostBindTable::from_ids([spark_ir::HostId::new("host", "ping", 1)]).unwrap()).unwrap();
    assert!(m.functions.iter().any(|f| f.code.iter().any(|&b| b == Op::CallHost as u8)));
    let mut vm = Vm::new(m);
    vm.prepare_host_slots(["host.ping"]);
    vm.register_native("host.ping", |_ctx, args| {
        let n = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
        Ok(Value::Number(n + 1.0))
    });
    let v = vm.run(&mut StdHost).unwrap();
    assert_eq!(v.as_number(), Some(8.0));
}
