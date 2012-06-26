//! Spark **脚本插件**机制。
//!
//! # 定位
//!
//! - **给 scripting / `spark-vm` 用**：宿主在启动时把插件装进 VM，脚本经 `CallNative` 调用。
//! - **不是** Rust 扩展点：Rust 代码需要能力时直接 `path` / crates.io 依赖对应 crate 即可。
//!
//! 插件只负责声明元信息，并把原生函数注册到 [`Vm`]。

use std::fmt;

use spark_vm::Vm;

/// 插件元信息（稳定 id，供注册表去重）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginInfo {
    pub id: &'static str,
    pub version: &'static str,
    pub description: &'static str,
}

/// 可安装到 VM 的脚本插件。
///
/// 实现方持有内部状态（通常 `Rc<RefCell<_>>`），在 [`Plugin::install`] 里
/// `register_native` 一批供脚本调用的函数。
pub trait Plugin {
    fn info(&self) -> PluginInfo;

    /// 本插件将注册的原生名（`'static`，供脚本前端编译期声明）。
    fn native_names(&self) -> &'static [&'static str];

    /// 向 VM 注册原生函数。可多次调用（每个模组 VM 各装一次）。
    fn install(&mut self, vm: &mut Vm);
}

/// 插件注册表错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum PluginError {
    DuplicateId { id: String },
    NotFound { id: String },
}

impl PluginError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateId { .. } => "spark.plugin.duplicate_id",
            Self::NotFound { .. } => "spark.plugin.not_found",
        }
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for PluginError {}

/// 汇总若干插件的原生名（去重保序）。
pub fn collect_native_names(plugins: &[&dyn Plugin]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for p in plugins {
        for n in p.native_names() {
            if !out.iter().any(|x| *x == *n) {
                out.push(*n);
            }
        }
    }
    out
}

/// 插件注册表：宿主持有，按需装进各个脚本 VM。
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), PluginError> {
        let id = plugin.info().id;
        if self.plugins.iter().any(|p| p.info().id == id) {
            return Err(PluginError::DuplicateId { id: id.into() });
        }
        tracing::info!(
            plugin = id,
            version = plugin.info().version,
            "已注册脚本插件"
        );
        self.plugins.push(plugin);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn infos(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(|p| p.info()).collect()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.plugins.iter().any(|p| p.info().id == id)
    }

    /// 供脚本前端 `compile_with(..., natives)` 使用的名字表。
    pub fn native_names(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for p in &self.plugins {
            for n in p.native_names() {
                if !out.iter().any(|x| *x == *n) {
                    out.push(*n);
                }
            }
        }
        out
    }

    /// 把全部插件的原生函数装进指定 VM。
    pub fn install_all(&mut self, vm: &mut Vm) {
        for p in &mut self.plugins {
            p.install(vm);
        }
    }

    /// 只安装指定 id。
    pub fn install_one(&mut self, id: &str, vm: &mut Vm) -> Result<(), PluginError> {
        let Some(p) = self.plugins.iter_mut().find(|p| p.info().id == id) else {
            return Err(PluginError::NotFound { id: id.into() });
        };
        p.install(vm);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_gc::Value;
    use spark_vm::{FuncProto, Module, NativeCtx, Op, StdHost};

    struct EchoPlugin;

    impl Plugin for EchoPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                id: "echo",
                version: "0.1.0",
                description: "测试回声",
            }
        }

        fn native_names(&self) -> &'static [&'static str] {
            &["echo_ping"]
        }

        fn install(&mut self, vm: &mut Vm) {
            vm.register_native("echo_ping", |_ctx: &mut NativeCtx<'_>, args: Vec<Value>| {
                Ok(args.into_iter().next().unwrap_or(Value::Null))
            });
        }
    }

    #[test]
    fn register_and_call() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(EchoPlugin)).unwrap();
        assert!(reg.contains("echo"));
        assert_eq!(reg.native_names(), ["echo_ping"]);

        let mut module = Module {
            functions: vec![],
            entry: 0,
            native_names: Vec::new(),
        };
        let ni = module.intern_native("echo_ping");
        let mut f = FuncProto::new("__main", 0);
        let c = f.add_const_number(7.0);
        f.emit(Op::LoadConst);
        f.emit_u16(c);
        f.emit(Op::CallNative);
        f.emit_u16(ni);
        f.emit_u8(1);
        f.emit(Op::Return);
        module.functions.push(f);

        let mut vm = Vm::new(module);
        reg.install_all(&mut vm);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(7.0));
    }
}
