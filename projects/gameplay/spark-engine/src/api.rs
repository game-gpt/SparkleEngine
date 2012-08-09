//! 模组脚本可用的内置原生 API。

use std::cell::RefCell;
use std::rc::Rc;

use spark_gc::{GcObject, Value};
use spark_vm::{NativeCtx, Vm, VmError};

use crate::command_buffer::ScriptCommandBuffer;
use crate::registry::RegValue;
use crate::vfs::ModVfs;
use crate::EngineShared;

/// 编译期声明的原生名（须与 [`install_builtins`] 一致）。
pub const ENGINE_NATIVES: &[&str] = &[
    "log",
    "register_hook",
    "registry_set",
    "registry_get",
    "mod_id",
    "asset_path",
    "queue_spawn",
    "queue_despawn",
    "queue_add_component",
    "queue_remove_component",
];

/// 文档用标记类型。
pub struct BuiltinApi;

/// 向脚本 VM 安装引擎原生函数。
pub fn install_builtins(
    vm: &mut Vm,
    shared: &Rc<RefCell<EngineShared>>,
    mod_id: &str,
    vfs: &ModVfs,
    commands: &Rc<RefCell<ScriptCommandBuffer>>,
) {
    let shared_log = Rc::clone(shared);
    vm.register_native("log", move |ctx, args| {
        let msg = args
            .first()
            .map(|v| value_to_string(ctx, v))
            .transpose()?
            .unwrap_or_else(|| "null".into());
        tracing::info!(target: "spark_mod", "{msg}");
        shared_log.borrow_mut().logs.push(msg);
        Ok(Value::Null)
    });

    let shared_hook = Rc::clone(shared);
    let mid = mod_id.to_string();
    vm.register_native("register_hook", move |ctx, args| {
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let hook = value_to_string(ctx, &args[0])?;
        let func = value_to_string(ctx, &args[1])?;
        shared_hook
            .borrow_mut()
            .hooks
            .register(hook, mid.clone(), func);
        Ok(Value::Null)
    });

    let shared_set = Rc::clone(shared);
    vm.register_native("registry_set", move |ctx, args| {
        if args.len() < 3 {
            return Err(VmError::ArityMismatch { expected: 3, got: args.len() as u16 });
        }
        let ns = value_to_string(ctx, &args[0])?;
        let key = value_to_string(ctx, &args[1])?;
        let val = value_to_reg(ctx, &args[2])?;
        shared_set.borrow_mut().registry.set(ns, key, val);
        Ok(Value::Null)
    });

    let shared_get = Rc::clone(shared);
    vm.register_native("registry_get", move |ctx, args| {
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let ns = value_to_string(ctx, &args[0])?;
        let key = value_to_string(ctx, &args[1])?;
        let v = shared_get
            .borrow()
            .registry
            .get(&ns, &key)
            .cloned()
            .unwrap_or(RegValue::Null);
        Ok(reg_to_value(ctx, &v))
    });

    let mid = mod_id.to_string();
    vm.register_native("mod_id", move |ctx, _args| {
        Ok(ctx.heap.alloc_string(mid.clone()))
    });

    let vfs = vfs.clone();
    vm.register_native("asset_path", move |ctx, args| {
        let rel = match args.first() {
            Some(v) => value_to_string(ctx, v)?,
            None => String::new(),
        };
        let path = vfs
            .resolve(&rel)
            .map_err(|_| VmError::BadNativeArg { name: "asset_path" })?;
        Ok(ctx.heap.alloc_string(path.to_string_lossy().into_owned()))
    });

    let cmds = Rc::clone(commands);
    vm.register_native("queue_spawn", move |ctx, args| {
        let archetype = args
            .first()
            .map(|v| value_to_string(ctx, v))
            .transpose()?
            .unwrap_or_default();
        if archetype.is_empty() {
            return Err(VmError::BadNativeArg { name: "queue_spawn" });
        }
        cmds.borrow_mut().spawn(archetype);
        Ok(Value::Null)
    });

    let cmds = Rc::clone(commands);
    vm.register_native("queue_despawn", move |_ctx, args| {
        let entity = args
            .first()
            .and_then(|v| v.as_number())
            .ok_or(VmError::BadNativeArg {
                name: "queue_despawn",
            })? as u64;
        cmds.borrow_mut().despawn(entity);
        Ok(Value::Null)
    });

    let cmds = Rc::clone(commands);
    vm.register_native("queue_add_component", move |ctx, args| {
        if args.len() < 2 {
            return Err(VmError::ArityMismatch {
                expected: 2,
                got: args.len() as u16,
            });
        }
        let entity = args[0]
            .as_number()
            .ok_or(VmError::BadNativeArg {
                name: "queue_add_component",
            })? as u64;
        let component = value_to_string(ctx, &args[1])?;
        cmds.borrow_mut().add_component(entity, component);
        Ok(Value::Null)
    });

    let cmds = Rc::clone(commands);
    vm.register_native("queue_remove_component", move |ctx, args| {
        if args.len() < 2 {
            return Err(VmError::ArityMismatch {
                expected: 2,
                got: args.len() as u16,
            });
        }
        let entity = args[0]
            .as_number()
            .ok_or(VmError::BadNativeArg {
                name: "queue_remove_component",
            })? as u64;
        let component = value_to_string(ctx, &args[1])?;
        cmds.borrow_mut().remove_component(entity, component);
        Ok(Value::Null)
    });
}

fn value_to_string(ctx: &NativeCtx<'_>, v: &Value) -> Result<String, VmError> {
    match v {
        Value::Null => Ok(String::new()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => {
            if *n == n.trunc() && n.abs() < 1e15 {
                Ok(format!("{}", *n as i64))
            } else {
                Ok(n.to_string())
            }
        }
        Value::Entity(id) => Ok(id.to_string()),
        Value::Func(i) => Ok(format!("fn:{i}")),
        Value::Handle(h) => match ctx.heap.get(*h) {
            Ok(GcObject::String(s)) => Ok(s.clone()),
            Ok(_) => Ok(format!("<object {}>", h.0)),
            Err(_) => Err(VmError::BadNativeArg { name: "handle" }),
        },
    }
}

fn value_to_reg(ctx: &NativeCtx<'_>, v: &Value) -> Result<RegValue, VmError> {
    Ok(match v {
        Value::Null => RegValue::Null,
        Value::Bool(b) => RegValue::Bool(*b),
        Value::Number(n) => RegValue::Number(*n),
        Value::Entity(e) => RegValue::Entity(*e),
        Value::Func(_) => {
            return Err(VmError::BadNativeArg { name: "function" });
        }
        Value::Handle(h) => match ctx.heap.get(*h) {
            Ok(GcObject::String(s)) => RegValue::String(s.clone()),
            Ok(_) => {
                return Err(VmError::BadNativeArg { name: "string_handle" });
            }
            Err(_) => return Err(VmError::BadNativeArg { name: "handle" }),
        },
    })
}

fn reg_to_value(ctx: &mut NativeCtx<'_>, v: &RegValue) -> Value {
    match v {
        RegValue::Null => Value::Null,
        RegValue::Bool(b) => Value::Bool(*b),
        RegValue::Number(n) => Value::Number(*n),
        RegValue::Entity(e) => Value::Entity(*e),
        RegValue::String(s) => ctx.heap.alloc_string(s.clone()),
    }
}
