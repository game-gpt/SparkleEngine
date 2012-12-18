//! [`Live2dPlugin`]：实现 `spark-plugin::Plugin`。

use std::{cell::RefCell, rc::Rc};

use spark_gc::{GcObject, Value};
use spark_plugin::{Plugin, PluginInfo};
use spark_vm::{NativeCtx, Vm, VmError};

use crate::{
    backend::NullLive2dBackend,
    runtime::{Live2dModelId, Live2dRuntime},
};

/// 脚本侧原生名。
pub const LIVE2D_NATIVES: &[&str] =
    &["live2d_load", "live2d_unload", "live2d_set_param", "live2d_get_param", "live2d_update", "live2d_start_motion"];

pub struct Live2dPlugin {
    runtime: Rc<RefCell<Live2dRuntime>>,
}

impl Live2dPlugin {
    pub fn new(runtime: Rc<RefCell<Live2dRuntime>>) -> Self {
        Self { runtime }
    }

    /// 使用占位后端。
    pub fn with_null_backend() -> Self {
        Self::new(Live2dRuntime::new(Box::new(NullLive2dBackend::default())))
    }

    pub fn runtime(&self) -> &Rc<RefCell<Live2dRuntime>> {
        &self.runtime
    }
}

impl Plugin for Live2dPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo { id: "live2d", version: "0.1.0", description: "Live2D 模型参数与动作（脚本插件）" }
    }

    fn native_names(&self) -> &'static [&'static str] {
        LIVE2D_NATIVES
    }

    fn install(&mut self, vm: &mut Vm) {
        let rt = Rc::clone(&self.runtime);
        vm.register_native("live2d_load", move |ctx, args| {
            let path = arg_string(ctx, &args, 0)?;
            let id = rt.borrow_mut().backend.load(&path).map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Number(id.0 as f64))
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("live2d_unload", move |_ctx, args| {
            let id = arg_model(&args, 0)?;
            rt.borrow_mut().backend.unload(id).map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("live2d_set_param", move |ctx, args| {
            let id = arg_model(&args, 0)?;
            let name = arg_string(ctx, &args, 1)?;
            let value = arg_f32(&args, 2)?;
            rt.borrow_mut().backend.set_param(id, &name, value).map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("live2d_get_param", move |ctx, args| {
            let id = arg_model(&args, 0)?;
            let name = arg_string(ctx, &args, 1)?;
            let v = rt.borrow().backend.get_param(id, &name).map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Number(v as f64))
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("live2d_update", move |_ctx, args| {
            let id = arg_model(&args, 0)?;
            let dt = args.get(1).and_then(|v| v.as_number()).map(|n| n as f32).unwrap_or(0.0);
            rt.borrow_mut().backend.update(id, dt).map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("live2d_start_motion", move |ctx, args| {
            let id = arg_model(&args, 0)?;
            let group = arg_string(ctx, &args, 1)?;
            let index = args.get(2).and_then(|v| v.as_number()).map(|n| n as i32).unwrap_or(0);
            rt.borrow_mut().backend.start_motion(id, &group, index).map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });
    }
}

fn arg_model(args: &[Value], i: usize) -> Result<Live2dModelId, VmError> {
    let n = args.get(i).and_then(|v| v.as_number()).ok_or_else(|| VmError::BadNativeArg { name: "model_id" })?;
    Ok(Live2dModelId(n as u32))
}

fn arg_f32(args: &[Value], i: usize) -> Result<f32, VmError> {
    args.get(i).and_then(|v| v.as_number()).map(|n| n as f32).ok_or_else(|| VmError::BadNativeArg { name: "number" })
}

fn arg_string(ctx: &NativeCtx<'_>, args: &[Value], i: usize) -> Result<String, VmError> {
    let v = args.get(i).ok_or_else(|| VmError::BadNativeArg { name: "string" })?;
    match v {
        Value::Handle(h) => match ctx.heap.get(*h) {
            Ok(GcObject::String(s)) => Ok(s.clone()),
            _ => Err(VmError::BadNativeArg { name: "string" }),
        },
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Ok(String::new()),
        _ => Err(VmError::BadNativeArg { name: "string" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_plugin::PluginRegistry;
    use spark_vm::{FuncProto, Module, Op, StdHost};

    #[test]
    fn null_backend_params() {
        let plugin = Live2dPlugin::with_null_backend();
        let rt = Rc::clone(plugin.runtime());
        let id = rt.borrow_mut().backend.load("model.model3.json").unwrap();
        rt.borrow_mut().backend.set_param(id, "ParamAngleX", 0.5).unwrap();
        let v = rt.borrow().backend.get_param(id, "ParamAngleX").unwrap();
        assert!((v - 0.5).abs() < 1e-5);
    }

    #[test]
    fn install_and_load_via_vm() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(Live2dPlugin::with_null_backend())).unwrap();
        assert!(reg.contains("live2d"));

        let mut module = Module { functions: vec![], entry: 0, native_names: vec!["live2d_load".into()] };
        let mut f = FuncProto::new("on_load", 0);
        let s = f.add_string("demo");
        f.emit(Op::LoadString);
        f.emit_u16(s);
        f.emit(Op::CallHost);
        f.emit_u16(0);
        f.emit_u8(1);
        f.emit(Op::Return);
        module.functions.push(f);

        let mut vm = spark_vm::Vm::new(module);
        vm.prepare_host_slots(["live2d_load"]);
        reg.install_all(&mut vm);
        let idv = vm.run(&mut StdHost).unwrap();
        assert_eq!(idv.as_number(), Some(0.0));
    }
}
