//! 模组脚本可用的内置原生 API。

use std::{cell::RefCell, rc::Rc};

use spark_gc::{GcObject, Value};
use spark_script::{HostEffect, HostFunction, HostFunctionId, HostPhase, HostSchema};
use spark_vm::{NativeCtx, Vm, VmError};

use crate::{
    EngineShared,
    access_policy::{check_host_determinism, check_host_phase},
    column_dispatch::BoundFieldDispatch,
    command_buffer::ScriptCommandBuffer,
    registry::RegValue,
    script_component_store::ColumnFieldValue,
    vfs::ModVfs,
};

/// 编译/装载声明的调度名（限定名，须与 [`install_builtins`] / [`engine_host_schema`] 一致）。
pub const ENGINE_NATIVES: &[&str] = &[
    "engine.log",
    "engine.register_hook",
    "engine.registry_set",
    "engine.registry_get",
    "engine.mod_id",
    "engine.asset_path",
    "engine.queue_spawn",
    "engine.queue_despawn",
    "engine.queue_add_component",
    "engine.queue_remove_component",
    "engine.query_archetype_count",
    "engine.query_entity_at",
    "engine.query_batch_count",
    "engine.query_batch_entity_at",
    "engine.query_column_f32",
    "engine.set_column_f32",
    "engine.query_column_i32",
    "engine.set_column_i32",
    "engine.query_column_bool",
    "engine.set_column_bool",
];

/// 文档用标记类型。
pub struct BuiltinApi;

/// 引擎内置宿主 ABI（阶段与效果进入 schema，供编译与运行门禁共用）。
pub fn engine_host_schema() -> HostSchema {
    let mut schema = HostSchema::new(1);
    let sim = [HostPhase::OnLoad, HostPhase::OnStart, HostPhase::FixedUpdate, HostPhase::Update, HostPhase::LateUpdate, HostPhase::OnEvent];
    let any = [HostPhase::Any];

    schema.insert(
        HostFunction::new(HostFunctionId::new("engine", "log", 1))
            .phases(any)
            .determinism(spark_script::DeterminismClass::Nondeterministic)
            .effect(HostEffect::Nondeterministic),
    );
    schema.insert(
        HostFunction::new(HostFunctionId::new("engine", "register_hook", 1))
            .phases([HostPhase::OnLoad, HostPhase::OnStart])
            .effect(HostEffect::WriteComponent),
    );
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "registry_set", 1)).phases(sim).effect(HostEffect::WriteComponent));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "registry_get", 1)).phases(any).effect(HostEffect::Pure));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "mod_id", 1)).phases(any).effect(HostEffect::Pure));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "asset_path", 1)).phases(any).effect(HostEffect::AssetRead));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "queue_spawn", 1)).phases(sim).effect(HostEffect::SpawnEntity));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "queue_despawn", 1)).phases(sim).effect(HostEffect::DespawnEntity));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "queue_add_component", 1)).phases(sim).effect(HostEffect::WriteComponent));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "queue_remove_component", 1)).phases(sim).effect(HostEffect::WriteComponent));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "query_archetype_count", 1)).phases(any).effect(HostEffect::ReadWorld));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "query_entity_at", 1)).phases(any).effect(HostEffect::ReadWorld));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "query_batch_count", 1)).phases(sim).effect(HostEffect::ReadWorld));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "query_batch_entity_at", 2)).phases(sim).effect(HostEffect::ReadWorld));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "query_column_f32", 4)).phases(sim).effect(HostEffect::ReadWorld));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "set_column_f32", 5)).phases(sim).effect(HostEffect::WriteComponent));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "query_column_i32", 4)).phases(sim).effect(HostEffect::ReadWorld));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "set_column_i32", 5)).phases(sim).effect(HostEffect::WriteComponent));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "query_column_bool", 4)).phases(sim).effect(HostEffect::ReadWorld));
    schema.insert(HostFunction::new(HostFunctionId::new("engine", "set_column_bool", 5)).phases(sim).effect(HostEffect::WriteComponent));
    schema
}

fn gate(shared: &EngineShared, import: &str) -> Result<(), VmError> {
    if !shared.execution_profile.enforces_runtime_gate() {
        return Ok(());
    }
    check_host_phase(&shared.host_schema, import, shared.active_phase).map_err(|detail| VmError::HostDenied { detail })?;
    check_host_determinism(&shared.host_schema, import, shared.active_determinism).map_err(|detail| VmError::HostDenied { detail })?;
    Ok(())
}

fn gate_component_write(shared: &EngineShared, component: &str) -> Result<(), VmError> {
    if !shared.execution_profile.enforces_runtime_gate() {
        return Ok(());
    }
    if shared.access.allows_write(component) {
        Ok(())
    }
    else {
        Err(VmError::HostDenied { detail: format!("host_access_denied:write:{component}") })
    }
}

fn gate_read_world(shared: &EngineShared) -> Result<(), VmError> {
    if !shared.execution_profile.enforces_runtime_gate() {
        return Ok(());
    }
    if shared.access.allows_read_world() { Ok(()) } else { Err(VmError::HostDenied { detail: "host_access_denied:read_world".into() }) }
}

fn gate_batch_read(shared: &EngineShared, import: &str) -> Result<(), VmError> {
    gate(shared, import)?;
    gate_read_world(shared)?;
    if shared.active_column_batch.is_none() {
        return Err(VmError::HostDenied { detail: "host_batch_denied:no_active_column_batch".into() });
    }
    Ok(())
}

fn gate_column_dispatch(shared: &EngineShared, import: &str) -> Result<(), VmError> {
    gate(shared, import)?;
    gate_read_world(shared)?;
    if shared.active_column_batch.is_none() {
        return Err(VmError::HostDenied { detail: "host_column_denied:no_active_column_batch".into() });
    }
    if shared.active_column_dispatch.is_none() {
        return Err(VmError::HostDenied { detail: "host_column_denied:no_dispatch_table".into() });
    }
    if shared.active_component_store.is_none() {
        return Err(VmError::HostDenied { detail: "host_column_denied:no_component_store".into() });
    }
    Ok(())
}

fn column_entity_bits(shared: &EngineShared, archetype_index: usize, row: usize) -> Result<u64, VmError> {
    let batch = shared.active_column_batch.as_ref().expect("gate_column_dispatch ensures batch");
    batch
        .entity_bits(shared.active_query(), archetype_index, row)
        .ok_or(VmError::BadNativeArg { name: "column_row" })
}

fn bound_field_dispatch(shared: &EngineShared, column_index: usize, field_index: usize) -> Result<BoundFieldDispatch, VmError> {
    let dispatch = shared.active_column_dispatch.as_ref().expect("gate_column_dispatch ensures dispatch");
    dispatch
        .field(column_index, field_index)
        .copied()
        .ok_or(VmError::BadNativeArg { name: "column_field" })
}

struct ColumnCoords {
    column_index: usize,
    archetype_index: usize,
    row: usize,
    field_index: usize,
}

fn parse_column_coords(args: &[Value], name: &'static str) -> Result<ColumnCoords, VmError> {
    if args.len() < 4 {
        return Err(VmError::ArityMismatch { expected: 4, got: args.len() as u16 });
    }
    Ok(ColumnCoords {
        column_index: args[0].as_number().ok_or(VmError::BadNativeArg { name })? as usize,
        archetype_index: args[1].as_number().ok_or(VmError::BadNativeArg { name })? as usize,
        row: args[2].as_number().ok_or(VmError::BadNativeArg { name })? as usize,
        field_index: args[3].as_number().ok_or(VmError::BadNativeArg { name })? as usize,
    })
}

fn gate_column_read(shared: &EngineShared, _import: &str, slot: crate::command_apply::ComponentDescriptorId) -> Result<(), VmError> {
    if !shared.execution_profile.enforces_runtime_gate() {
        return Ok(());
    }
    let name = shared.active_component_catalog.name_of(slot).unwrap_or("?");
    if !shared.access.allows_read_component(name) {
        return Err(VmError::HostDenied { detail: format!("host_access_denied:read_column:{name}") });
    }
    Ok(())
}

fn resolve_column_write(
    shared: &mut EngineShared,
    import: &str,
    coords: ColumnCoords,
) -> Result<(BoundFieldDispatch, u64), VmError> {
    gate_column_dispatch(shared, import)?;
    let field = bound_field_dispatch(shared, coords.column_index, coords.field_index)?;
    if !field.column_write {
        return Err(VmError::HostDenied { detail: "host_column_denied:read_only_column".into() });
    }
    if shared.execution_profile.enforces_runtime_gate() {
        let name = shared.active_component_catalog.name_of(field.slot).unwrap_or("?");
        gate_component_write(shared, name)?;
    }
    let entity_bits = column_entity_bits(shared, coords.archetype_index, coords.row)?;
    Ok((field, entity_bits))
}

/// 向脚本 VM 安装引擎原生函数。
pub fn install_builtins(
    vm: &mut Vm,
    shared: &Rc<RefCell<EngineShared>>,
    mod_id: &str,
    vfs: &ModVfs,
    commands: &Rc<RefCell<ScriptCommandBuffer>>,
) {
    let shared_log = Rc::clone(shared);
    vm.register_native("engine.log", move |ctx, args| {
        gate(&shared_log.borrow(), "engine.log")?;
        let msg = args.first().map(|v| value_to_string(ctx, v)).transpose()?.unwrap_or_else(|| "null".into());
        tracing::info!(target: "spark_mod", "{msg}");
        shared_log.borrow_mut().logs.push(msg);
        Ok(Value::Null)
    });

    let shared_hook = Rc::clone(shared);
    let mid = mod_id.to_string();
    vm.register_native("engine.register_hook", move |ctx, args| {
        gate(&shared_hook.borrow(), "engine.register_hook")?;
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let hook = value_to_string(ctx, &args[0])?;
        let func = value_to_string(ctx, &args[1])?;
        shared_hook.borrow_mut().hooks.register(hook, mid.clone(), func);
        Ok(Value::Null)
    });

    let shared_set = Rc::clone(shared);
    vm.register_native("engine.registry_set", move |ctx, args| {
        gate(&shared_set.borrow(), "engine.registry_set")?;
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
    vm.register_native("engine.registry_get", move |ctx, args| {
        gate(&shared_get.borrow(), "engine.registry_get")?;
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let ns = value_to_string(ctx, &args[0])?;
        let key = value_to_string(ctx, &args[1])?;
        let v = shared_get.borrow().registry.get(&ns, &key).cloned().unwrap_or(RegValue::Null);
        Ok(reg_to_value(ctx, &v))
    });

    let shared_mid = Rc::clone(shared);
    let mid = mod_id.to_string();
    vm.register_native("engine.mod_id", move |ctx, _args| {
        gate(&shared_mid.borrow(), "engine.mod_id")?;
        Ok(ctx.heap.alloc_string(mid.clone()))
    });

    let shared_asset = Rc::clone(shared);
    let vfs = vfs.clone();
    vm.register_native("engine.asset_path", move |ctx, args| {
        gate(&shared_asset.borrow(), "engine.asset_path")?;
        let rel = match args.first() {
            Some(v) => value_to_string(ctx, v)?,
            None => String::new(),
        };
        let path = vfs.resolve(&rel).map_err(|_| VmError::BadNativeArg { name: "asset_path" })?;
        Ok(ctx.heap.alloc_string(path.to_string_lossy().into_owned()))
    });

    let shared_spawn = Rc::clone(shared);
    let cmds = Rc::clone(commands);
    vm.register_native("engine.queue_spawn", move |ctx, args| {
        gate(&shared_spawn.borrow(), "engine.queue_spawn")?;
        let archetype = args.first().map(|v| value_to_string(ctx, v)).transpose()?.unwrap_or_default();
        if archetype.is_empty() {
            return Err(VmError::BadNativeArg { name: "queue_spawn" });
        }
        cmds.borrow_mut().spawn(archetype);
        Ok(Value::Null)
    });

    let shared_despawn = Rc::clone(shared);
    let cmds = Rc::clone(commands);
    vm.register_native("engine.queue_despawn", move |_ctx, args| {
        gate(&shared_despawn.borrow(), "engine.queue_despawn")?;
        let entity = args.first().and_then(|v| v.as_number()).ok_or(VmError::BadNativeArg { name: "queue_despawn" })? as u64;
        cmds.borrow_mut().despawn(entity);
        Ok(Value::Null)
    });

    let shared_add = Rc::clone(shared);
    let cmds = Rc::clone(commands);
    vm.register_native("engine.queue_add_component", move |ctx, args| {
        gate(&shared_add.borrow(), "engine.queue_add_component")?;
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let entity = args[0].as_number().ok_or(VmError::BadNativeArg { name: "queue_add_component" })? as u64;
        let component = value_to_string(ctx, &args[1])?;
        gate_component_write(&shared_add.borrow(), &component)?;
        cmds.borrow_mut().add_component(entity, component);
        Ok(Value::Null)
    });

    let shared_rm = Rc::clone(shared);
    let cmds = Rc::clone(commands);
    vm.register_native("engine.queue_remove_component", move |ctx, args| {
        gate(&shared_rm.borrow(), "engine.queue_remove_component")?;
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let entity = args[0].as_number().ok_or(VmError::BadNativeArg { name: "queue_remove_component" })? as u64;
        let component = value_to_string(ctx, &args[1])?;
        gate_component_write(&shared_rm.borrow(), &component)?;
        cmds.borrow_mut().remove_component(entity, component);
        Ok(Value::Null)
    });

    let shared_q = Rc::clone(shared);
    vm.register_native("engine.query_archetype_count", move |ctx, args| {
        gate(&shared_q.borrow(), "engine.query_archetype_count")?;
        gate_read_world(&shared_q.borrow())?;
        let name = args.first().map(|v| value_to_string(ctx, v)).transpose()?.unwrap_or_default();
        let shared = shared_q.borrow();
        if !shared.access.allows_query_archetype(&name) {
            return Ok(Value::Number(0.0));
        }
        let n = shared.active_query().count(&name);
        Ok(Value::Number(n as f64))
    });

    let shared_e = Rc::clone(shared);
    vm.register_native("engine.query_entity_at", move |ctx, args| {
        gate(&shared_e.borrow(), "engine.query_entity_at")?;
        gate_read_world(&shared_e.borrow())?;
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let name = value_to_string(ctx, &args[0])?;
        let index = args[1].as_number().ok_or(VmError::BadNativeArg { name: "query_entity_at" })? as usize;
        let shared = shared_e.borrow();
        if !shared.access.allows_query_archetype(&name) {
            return Ok(Value::Null);
        }
        match shared.active_query().entity_at(&name, index) {
            Some(bits) => Ok(Value::Entity(bits)),
            None => Ok(Value::Null),
        }
    });

    let shared_batch_count = Rc::clone(shared);
    vm.register_native("engine.query_batch_count", move |_ctx, args| {
        gate_batch_read(&shared_batch_count.borrow(), "engine.query_batch_count")?;
        if args.is_empty() {
            return Err(VmError::ArityMismatch { expected: 1, got: 0 });
        }
        let archetype_index = args[0].as_number().ok_or(VmError::BadNativeArg { name: "query_batch_count" })? as usize;
        let shared = shared_batch_count.borrow();
        let batch = shared.active_column_batch.as_ref().expect("gate_batch_read ensures batch");
        Ok(Value::Number(batch.entity_count(shared.active_query(), archetype_index) as f64))
    });

    let shared_batch_entity = Rc::clone(shared);
    vm.register_native("engine.query_batch_entity_at", move |_ctx, args| {
        gate_batch_read(&shared_batch_entity.borrow(), "engine.query_batch_entity_at")?;
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let archetype_index = args[0].as_number().ok_or(VmError::BadNativeArg { name: "query_batch_entity_at" })? as usize;
        let row = args[1].as_number().ok_or(VmError::BadNativeArg { name: "query_batch_entity_at" })? as usize;
        let shared = shared_batch_entity.borrow();
        let batch = shared.active_column_batch.as_ref().expect("gate_batch_read ensures batch");
        let bits = batch.entity_bits(shared.active_query(), archetype_index, row);
        Ok(bits.map(Value::Entity).unwrap_or(Value::Null))
    });

    let shared_col_read = Rc::clone(shared);
    vm.register_native("engine.query_column_f32", move |_ctx, args| {
        let import = "engine.query_column_f32";
        let coords = parse_column_coords(&args, import)?;
        let shared = shared_col_read.borrow();
        gate_column_dispatch(&shared, import)?;
        let field = bound_field_dispatch(&shared, coords.column_index, coords.field_index)?;
        gate_column_read(&shared, import, field.slot)?;
        let entity_bits = column_entity_bits(&shared, coords.archetype_index, coords.row)?;
        let store = shared.active_component_store.as_ref().expect("gate_column_dispatch ensures store");
        let value = store
            .read_dispatch(&field, entity_bits)
            .ok_or(VmError::BadNativeArg { name: "query_column_f32" })?;
        match value {
            ColumnFieldValue::F32(v) => Ok(Value::Number(v as f64)),
            _ => Err(VmError::BadNativeArg { name: "query_column_f32" }),
        }
    });

    let shared_col_write = Rc::clone(shared);
    vm.register_native("engine.set_column_f32", move |_ctx, args| {
        let import = "engine.set_column_f32";
        if args.len() < 5 {
            return Err(VmError::ArityMismatch { expected: 5, got: args.len() as u16 });
        }
        let coords = parse_column_coords(&args, import)?;
        let value = args[4].as_number().ok_or(VmError::BadNativeArg { name: import })? as f32;
        let mut shared = shared_col_write.borrow_mut();
        let (field, entity_bits) = resolve_column_write(&mut shared, import, coords)?;
        let store = shared.active_component_store.as_mut().expect("gate_column_dispatch ensures store");
        if !store.write_dispatch(&field, entity_bits, ColumnFieldValue::F32(value)) {
            return Err(VmError::BadNativeArg { name: import });
        }
        Ok(Value::Null)
    });

    let shared_col_read_i32 = Rc::clone(shared);
    vm.register_native("engine.query_column_i32", move |_ctx, args| {
        let import = "engine.query_column_i32";
        let coords = parse_column_coords(&args, import)?;
        let shared = shared_col_read_i32.borrow();
        gate_column_dispatch(&shared, import)?;
        let field = bound_field_dispatch(&shared, coords.column_index, coords.field_index)?;
        gate_column_read(&shared, import, field.slot)?;
        let entity_bits = column_entity_bits(&shared, coords.archetype_index, coords.row)?;
        let store = shared.active_component_store.as_ref().expect("gate_column_dispatch ensures store");
        let value = store
            .read_dispatch(&field, entity_bits)
            .ok_or(VmError::BadNativeArg { name: import })?;
        match value {
            ColumnFieldValue::I32(v) => Ok(Value::Number(v as f64)),
            _ => Err(VmError::BadNativeArg { name: import }),
        }
    });

    let shared_col_write_i32 = Rc::clone(shared);
    vm.register_native("engine.set_column_i32", move |_ctx, args| {
        let import = "engine.set_column_i32";
        if args.len() < 5 {
            return Err(VmError::ArityMismatch { expected: 5, got: args.len() as u16 });
        }
        let coords = parse_column_coords(&args, import)?;
        let value = args[4].as_number().ok_or(VmError::BadNativeArg { name: import })? as i32;
        let mut shared = shared_col_write_i32.borrow_mut();
        let (field, entity_bits) = resolve_column_write(&mut shared, import, coords)?;
        let store = shared.active_component_store.as_mut().expect("gate_column_dispatch ensures store");
        if !store.write_dispatch(&field, entity_bits, ColumnFieldValue::I32(value)) {
            return Err(VmError::BadNativeArg { name: import });
        }
        Ok(Value::Null)
    });

    let shared_col_read_bool = Rc::clone(shared);
    vm.register_native("engine.query_column_bool", move |_ctx, args| {
        let import = "engine.query_column_bool";
        let coords = parse_column_coords(&args, import)?;
        let shared = shared_col_read_bool.borrow();
        gate_column_dispatch(&shared, import)?;
        let field = bound_field_dispatch(&shared, coords.column_index, coords.field_index)?;
        gate_column_read(&shared, import, field.slot)?;
        let entity_bits = column_entity_bits(&shared, coords.archetype_index, coords.row)?;
        let store = shared.active_component_store.as_ref().expect("gate_column_dispatch ensures store");
        let value = store
            .read_dispatch(&field, entity_bits)
            .ok_or(VmError::BadNativeArg { name: import })?;
        match value {
            ColumnFieldValue::Bool(v) => Ok(Value::Bool(v)),
            _ => Err(VmError::BadNativeArg { name: import }),
        }
    });

    let shared_col_write_bool = Rc::clone(shared);
    vm.register_native("engine.set_column_bool", move |_ctx, args| {
        let import = "engine.set_column_bool";
        if args.len() < 5 {
            return Err(VmError::ArityMismatch { expected: 5, got: args.len() as u16 });
        }
        let coords = parse_column_coords(&args, import)?;
        let value = value_to_bool(&args[4])?;
        let mut shared = shared_col_write_bool.borrow_mut();
        let (field, entity_bits) = resolve_column_write(&mut shared, import, coords)?;
        let store = shared.active_component_store.as_mut().expect("gate_column_dispatch ensures store");
        if !store.write_dispatch(&field, entity_bits, ColumnFieldValue::Bool(value)) {
            return Err(VmError::BadNativeArg { name: import });
        }
        Ok(Value::Null)
    });
}

fn value_to_bool(v: &Value) -> Result<bool, VmError> {
    match v {
        Value::Bool(b) => Ok(*b),
        Value::Number(n) => Ok(*n != 0.0),
        _ => Err(VmError::BadNativeArg { name: "bool" }),
    }
}

fn value_to_string(ctx: &NativeCtx<'_>, v: &Value) -> Result<String, VmError> {
    match v {
        Value::Null => Ok(String::new()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => {
            if *n == n.trunc() && n.abs() < 1e15 {
                Ok(format!("{}", *n as i64))
            }
            else {
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
