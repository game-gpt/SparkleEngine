# spark-vm

栈式字节码解释器。

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

也可用 `StdHost`、`register_native`、`call_function`、`verify_bytecode`。VM 不持有 ECS `World`。错误：`VmError`、
`BytecodeVerifyError`。字节码通常由 `spark-script` / `spark-ir` 生成。

```bash
cargo test -p spark-vm
```
