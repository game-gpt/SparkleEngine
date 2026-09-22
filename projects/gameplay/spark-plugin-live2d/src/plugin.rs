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

/// Live2D 脚本插件：把 [`LIVE2D_NATIVES`] 注册进 VM。
pub struct Live2dPlugin {
    runtime: Rc<RefCell<Live2dRuntime>>,
}

impl Live2dPlugin {
    /// 使用调用方提供的共享运行时。
    pub fn new(runtime: Rc<RefCell<Live2dRuntime>>) -> Self {
        Self { runtime }
    }

    /// 使用占位后端。
    pub fn with_null_backend() -> Self {
        Self::new(Live2dRuntime::new(Box::new(NullLive2dBackend::default())))
    }

    /// 借出共享运行时（便于宿主直接调后端）。
    pub fn runtime(&self) -> &Rc<RefCell<Live2dRuntime>> {
        &self.runtime
    }
}

impl Plugin for Live2dPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo { id: "live2d", version: "0.0.0", description: "Live2D 模型参数与动作（脚本插件）" }
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
