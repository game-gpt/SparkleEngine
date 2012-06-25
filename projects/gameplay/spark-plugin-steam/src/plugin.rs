//! [`SteamPlugin`]：实现 `spark-plugin::Plugin`。

use std::cell::RefCell;
use std::rc::Rc;

use spark_gc::{GcObject, Value};
use spark_plugin::{Plugin, PluginInfo};
use spark_vm::{NativeCtx, Vm, VmError};

use crate::backend::NullSteamBackend;
use crate::runtime::SteamRuntime;

/// 脚本侧原生名。
pub const STEAM_NATIVES: &[&str] = &[
    "steam_is_available",
    "steam_app_id",
    "steam_user_name",
    "steam_achievement_unlock",
    "steam_achievement_unlocked",
    "steam_achievement_clear",
    "steam_stat_get",
    "steam_stat_set",
    "steam_stats_store",
    "steam_cloud_read",
    "steam_cloud_write",
    "steam_cloud_delete",
    "steam_overlay_open_url",
];

pub struct SteamPlugin {
    runtime: Rc<RefCell<SteamRuntime>>,
}

impl SteamPlugin {
    pub fn new(runtime: Rc<RefCell<SteamRuntime>>) -> Self {
        Self { runtime }
    }

    pub fn with_null_backend() -> Self {
        Self::new(SteamRuntime::new(Box::new(NullSteamBackend::default())))
    }

    pub fn with_null_backend_app(app_id: u32, user_name: impl Into<String>) -> Self {
        Self::new(SteamRuntime::new(Box::new(NullSteamBackend::new(
            app_id, user_name,
        ))))
    }

    pub fn runtime(&self) -> &Rc<RefCell<SteamRuntime>> {
        &self.runtime
    }
}

impl Plugin for SteamPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "steam",
            version: "0.1.0",
            description: "Steam 成就 / 统计 / 云文件（脚本插件）",
        }
    }

    fn native_names(&self) -> &'static [&'static str] {
        STEAM_NATIVES
    }

    fn install(&mut self, vm: &mut Vm) {
        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_is_available", move |_ctx, _args| {
            Ok(Value::Bool(rt.borrow().backend.is_available()))
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_app_id", move |_ctx, _args| {
            Ok(Value::Number(rt.borrow().backend.app_id() as f64))
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_user_name", move |ctx, _args| {
            let name = rt.borrow().backend.user_name();
            Ok(ctx.heap.alloc_string(name))
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_achievement_unlock", move |ctx, args| {
            let id = arg_string(ctx, &args, 0)?;
            rt.borrow_mut()
                .backend
                .unlock_achievement(&id)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_achievement_unlocked", move |ctx, args| {
            let id = arg_string(ctx, &args, 0)?;
            let ok = rt
                .borrow()
                .backend
                .is_achievement_unlocked(&id)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Bool(ok))
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_achievement_clear", move |ctx, args| {
            let id = arg_string(ctx, &args, 0)?;
            rt.borrow_mut()
                .backend
                .clear_achievement(&id)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_stat_get", move |ctx, args| {
            let name = arg_string(ctx, &args, 0)?;
            let v = rt
                .borrow()
                .backend
                .get_stat(&name)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Number(v as f64))
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_stat_set", move |ctx, args| {
            let name = arg_string(ctx, &args, 0)?;
            let value = arg_f32(&args, 1)?;
            rt.borrow_mut()
                .backend
                .set_stat(&name, value)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_stats_store", move |_ctx, _args| {
            rt.borrow_mut()
                .backend
                .store_stats()
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_cloud_read", move |ctx, args| {
            let path = arg_string(ctx, &args, 0)?;
            match rt
                .borrow()
                .backend
                .cloud_read(&path)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?
            {
                Some(s) => Ok(ctx.heap.alloc_string(s)),
                None => Ok(Value::Null),
            }
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_cloud_write", move |ctx, args| {
            let path = arg_string(ctx, &args, 0)?;
            let data = arg_string(ctx, &args, 1)?;
            rt.borrow_mut()
                .backend
                .cloud_write(&path, &data)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_cloud_delete", move |ctx, args| {
            let path = arg_string(ctx, &args, 0)?;
            let ok = rt
                .borrow_mut()
                .backend
                .cloud_delete(&path)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Bool(ok))
        });

        let rt = Rc::clone(&self.runtime);
        vm.register_native("steam_overlay_open_url", move |ctx, args| {
            let url = arg_string(ctx, &args, 0)?;
            rt.borrow_mut()
                .backend
                .overlay_open_url(&url)
                .map_err(|_| VmError::BadNativeArg { name: "backend" })?;
            Ok(Value::Null)
        });
    }
}

fn arg_f32(args: &[Value], i: usize) -> Result<f32, VmError> {
    args.get(i)
        .and_then(|v| v.as_number())
        .map(|n| n as f32)
        .ok_or_else(|| VmError::BadNativeArg { name: "number" })
}

fn arg_string(ctx: &NativeCtx<'_>, args: &[Value], i: usize) -> Result<String, VmError> {
    let v = args
        .get(i)
        .ok_or_else(|| VmError::BadNativeArg { name: "string" })?;
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
    fn null_backend_achievements_and_cloud() {
        let plugin = SteamPlugin::with_null_backend_app(480, "tester");
        let rt = Rc::clone(plugin.runtime());
        assert!(!rt.borrow().backend.is_available());
        assert_eq!(rt.borrow().backend.app_id(), 480);
        rt.borrow_mut()
            .backend
            .unlock_achievement("ACH_FIRST")
            .unwrap();
        assert!(rt
            .borrow()
            .backend
            .is_achievement_unlocked("ACH_FIRST")
            .unwrap());
        rt.borrow_mut().backend.set_stat("kills", 3.0).unwrap();
        assert!((rt.borrow().backend.get_stat("kills").unwrap() - 3.0).abs() < 1e-5);
        rt.borrow_mut()
            .backend
            .cloud_write("save.txt", "hello")
            .unwrap();
        assert_eq!(
            rt.borrow().backend.cloud_read("save.txt").unwrap().as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn install_and_call_app_id() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(SteamPlugin::with_null_backend_app(1234, "u")))
            .unwrap();
        assert!(reg.contains("steam"));

        let mut module = Module {
            functions: vec![],
            entry: 0,
            native_names: Vec::new(),
        };
        let ni = module.intern_native("steam_app_id");
        let mut f = FuncProto::new("__main", 0);
        f.emit(Op::CallNative);
        f.emit_u16(ni);
        f.emit_u8(0);
        f.emit(Op::Return);
        module.functions.push(f);

        let mut vm = spark_vm::Vm::new(module);
        reg.install_all(&mut vm);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(1234.0));
    }
}
