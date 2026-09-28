# spark-vm

Stack bytecode interpreter.

```rust
use spark_vm::{FuncProto, HostHooks, Module, Op, Vm};

struct BufHost(String);
impl HostHooks for BufHost {
    fn print(&mut self, text: &str) {
        self.0.push_str(text);
        self.0.push('\n');
    }
}

let mut f = FuncProto::new("main", 0);
let c1 = f.add_const_number(40.0);
let c2 = f.add_const_number(2.0);
f.emit(Op::LoadConst);
f.emit_u16(c1);
f.emit(Op::LoadConst);
f.emit_u16(c2);
f.emit(Op::Add);
f.emit(Op::Return);

let mut vm = Vm::new(Module {
    functions: vec![f],
    entry: 0,
    native_names: Vec::new(),
});
let v = vm.run(&mut BufHost(String::new())).unwrap();
assert_eq!(v.as_number(), Some(42.0));
```

Also `StdHost`, `register_native`, `call_function`, `verify_bytecode`. VM does not hold an ECS `World`. Errors: `VmError`, `BytecodeVerifyError`. Bytecode is usually produced by `spark-script` / `spark-ir`.

```bash
cargo test -p spark-vm
```
