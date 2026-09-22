//! Spark **脚本插件**机制。
//!
//! # 定位
//!
//! - **给 scripting / `spark-vm` 用**：宿主在启动时把插件装进 VM，脚本经编译期绑定的 [`spark_vm::Op::CallHost`] 槽位调用。
//! - **不是** Rust 扩展点：Rust 代码需要能力时直接 `path` / crates.io 依赖对应 crate 即可。
//!
//! 插件只负责声明元信息，并把原生函数注册到 [`Vm`]。

#![warn(missing_docs)]
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

    pub fn args(&self) -> spark_diagnostics::ErrorArgs {
        use spark_diagnostics::{ErrorArg, ErrorArgs};
        use std::sync::Arc;
        match self {
            Self::DuplicateId { id } | Self::NotFound { id } => ErrorArgs::new().with("id", ErrorArg::String(Arc::from(id.as_str()))),
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
        tracing::info!(event = "spark.plugin.registered", plugin = id, version = plugin.info().version,);
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

    /// 把全部插件的原生函数装进指定 VM（调度键为 `plugin.<short>`）。
    pub fn install_all(&mut self, vm: &mut Vm) {
        for p in &mut self.plugins {
            let names: Vec<&'static str> = p.native_names().to_vec();
            p.install(vm);
            for n in names {
                let qualified = format!("plugin.{n}");
                if let Some(native) = vm.take_native(n) {
                    vm.register_native_fn(qualified, native);
                }
            }
        }
    }

    /// 只安装指定 id。
    pub fn install_one(&mut self, id: &str, vm: &mut Vm) -> Result<(), PluginError> {
        let Some(p) = self.plugins.iter_mut().find(|p| p.info().id == id)
        else {
            return Err(PluginError::NotFound { id: id.into() });
        };
        let names: Vec<&'static str> = p.native_names().to_vec();
        p.install(vm);
        for n in names {
            let qualified = format!("plugin.{n}");
            if let Some(native) = vm.take_native(n) {
                vm.register_native_fn(qualified, native);
            }
        }
        Ok(())
    }
}
