//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script_ruby::*;

use spark_gc::Value;
use spark_vm::{Op, StdHost, Vm};

#[test]
fn arithmetic_main() {
    let module = compile("return 40 + 2").unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(42.0));
}

#[test]
fn arithmetic_via_ir_has_no_call_native() {
    let module = compile("return 40 + 2").unwrap();
    assert!(module.functions.iter().all(|f| { !f.code.iter().any(|&b| b == Op::CallNative as u8 || b == Op::CallHost as u8) }));
}

#[test]
fn local_and_if_via_ir() {
    let module = compile(
        r#"
        x = 1
        if x < 2
          return 42
        else
          return 0
        end
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(42.0));
}

#[test]
fn while_via_ir() {
    let module = compile(
        r#"
        n = 0
        while n < 3
          n = n + 1
        end
        return n
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(3.0));
}

#[test]
fn until_via_ir() {
    let module = compile(
        r#"
        n = 0
        until n >= 3
          n = n + 1
        end
        return n
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(3.0));
}

#[test]
fn and_or_via_ir() {
    let module = compile(
        r#"
        a = false
        b = 7
        if a && b
          return 1
        end
        if a || b
          return 42
        end
        return 0
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(42.0));
}

#[test]
fn for_range_via_ir() {
    let module = compile(
        r#"
        n = 0
        for i in 1..3
          n = n + i
        end
        return n
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(6.0));
}

#[test]
fn break_via_ir() {
    let module = compile(
        r#"
        n = 0
        while true
          n = n + 1
          if n >= 3
            break
          end
        end
        return n
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(3.0));
}

#[test]
fn method_via_ir() {
    let module = compile(
        r#"
        def add(a, b)
          return a + b
        end
        return add(40, 2)
        "#,
    )
    .unwrap();
    assert!(module.functions.iter().any(|f| f.name == "add"), "expected IR method proto");
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(42.0));
}

#[test]
fn host_call_via_ir() {
    let module = compile_with_binds("return ping(7)", &HostBindTable::from_ids([spark_ir::HostId::new("host", "ping", 1)]).unwrap()).unwrap();
    assert!(module.functions.iter().any(|f| f.code.iter().any(|&b| b == Op::CallHost as u8)));
    let mut vm = Vm::new(module);
    vm.prepare_host_slots(["host.ping"]);
    vm.register_native("host.ping", |_ctx, args| {
        let n = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
        Ok(Value::Number(n + 1.0))
    });
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(8.0));
}

#[test]
fn method_call() {
    let module = compile(
        r#"
        def add(a, b)
          return a + b
        end
        return add(40, 2)
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(42.0));
}

#[test]
fn hex_literal_via_ir() {
    let module = compile(
        r#"
        return 0x2A
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(42.0));
}

#[test]
fn receiver_call_is_unsupported() {
    let err = compile(
        r#"
        def Foo_bar
          return 7
        end
        return Foo.bar()
        "#,
    )
    .unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
}

#[test]
fn class_is_unsupported() {
    let err = compile(
        r#"
        class Counter
          def initialize
            @n = 0
          end
          def bump
            @n = @n + 1
            return @n
          end
        end
        $c = Counter.new()
        return $c.bump()
        "#,
    )
    .unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
}

#[test]
fn each_block_is_unsupported() {
    let err = compile(
        r#"
        xs = [10, 20]
        s = 0
        xs.each{|v|
          s = s + v
        }
        return s
        "#,
    )
    .unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
}

#[test]
fn for_range_still_works_without_each() {
    let module = compile(
        r#"
        s = 0
        for i in 1..3
          s = s + i
        end
        return s
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(6.0));
}

#[test]
fn break_on_scene_nil() {
    let module = compile_with_binds(
        r#"
        n = 0
        while n < 5
          tick()
          n = n + 1
        end
        return n
        "#,
        &HostBindTable::from_ids([spark_ir::HostId::new("host", "tick", 1)]).unwrap(),
    )
    .unwrap();
    let mut vm = Vm::new(module);
    vm.prepare_host_slots(["host.tick"]);
    let frames = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let frames2 = frames.clone();
    vm.register_native("host.tick", move |_ctx, _args| {
        frames2.set(frames2.get() + 1);
        Ok(Value::Null)
    });
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(5.0));
    assert_eq!(frames.get(), 5);
}

#[test]
fn qualified_host_call_is_unsupported_without_ir() {
    // `Graphics.update` 需接收者调用；公共 IR 未覆盖前必须明确失败。
    let err = compile_with_binds(
        r#"
        i = 0
        while i < 3
          Graphics.update
          i = i + 1
        end
        return i
        "#,
        &HostBindTable::from_ids([spark_ir::HostId::new("host", "Graphics_update", 1)]).unwrap(),
    )
    .unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
}

#[test]
fn for_large_range_finishes() {
    let module = compile(
        r#"
        n = 0
        for i in 0..6000
          n = n + 1
        end
        return n
        "#,
    )
    .unwrap();
    let mut vm = Vm::new(module);
    vm.step_limit = 50_000_000;
    let value = vm.run(&mut StdHost).unwrap();
    assert_eq!(value.as_number(), Some(6001.0));
}

#[test]
fn default_param_class_is_unsupported() {
    let err = compile(
        r#"
        class Game_Variables
          def initialize(base=false)
            @data = []
            @plus = Game_Variables.new(true) unless base
          end
        end
        $v = Game_Variables.new
        return 1
        "#,
    )
    .unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
}
