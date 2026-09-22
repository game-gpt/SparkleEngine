# spark-jit

对 `FuncProto` 做字节码特化（如常量折叠）。当前不发射机器码。

```rust
use spark_jit::specialize_func;
use spark_vm::{FuncProto, Module, Op, StdHost, Vm};

let mut f = FuncProto::new("main", 0);
let a = f.add_const_number(20.0);
let b = f.add_const_number(22.0);
f.emit(Op::LoadConst);
f.emit_u16(a);
f.emit(Op::LoadConst);
f.emit_u16(b);
f.emit(Op::Add);
f.emit(Op::Return);
specialize_func(&mut f).unwrap();

let v = Vm::new(Module {
    functions: vec![f],
    entry: 0,
    native_names: Vec::new(),
})
.run(&mut StdHost)
.unwrap();
assert_eq!(v.as_number(), Some(42.0));
```

还有 `JitEngine`、`specialize_module`、`optimize_hot`。错误：`JitError`。

```bash
cargo test -p spark-jit
```
