//! Spark 引擎壳：帧主循环编排 + ECS 宿主桥 + 在 `spark-vm` / `spark-script` 之上的 **modder** 能力。
//!
//! 提供：固定步 / update·draw 相位编排、[`EcsHost3d`]（`Schedule` ↔ `GameHost3d`）、
//! 模组清单与发现、依赖排序加载、脚本入口、命名钩子、
//! 通用数据表、模组资源路径、脚本插件挂载（[`PluginRegistry`]）。模组逻辑一律跑在
//! [`spark_vm`]（经 [`spark_script`] 多前端编译），与宿主目标平台无关。
//! **不**拥有窗口后端（winit 等止于 `spark-renderer-wgpu` / 绑定宿主）。
//! **不**提供游戏内容权威（方块 / 配方等由游戏仓解释 [`DataRegistry`]）。
//! Rust 宿主若直接需要能力，请 path 依赖对应 crate，勿把 Rust API 伪装成插件。

mod api;
mod ecs_host;
mod frame;
mod hooks;
mod loader;
mod localization;
mod manifest;
mod registry;
mod run;
mod vfs;

pub use api::{BuiltinApi, ENGINE_NATIVES};
pub use ecs_host::{DrawBuffer3d, EcsHost3d, FrameSnapshot};
pub use frame::{
    FrameLoop, FrameLoopConfig, LoopedHost2d, LoopedHost3d, StepMode,
};
pub use hooks::{HookBus, HookRef};
pub use loader::{LoadedMod, ModLoader};
pub use localization::LocalizationService;
pub use manifest::{ManifestParseError, ModManifest};
pub use registry::{DataRegistry, RegValue};
pub use run::{run_ecs_game_3d, run_game, run_game_3d, run_game_3d_with, run_game_with};
pub use spark_plugin::{Plugin, PluginError, PluginInfo, PluginRegistry};
pub use vfs::ModVfs;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use spark_core::SparkError;
use spark_gc::Value;
use spark_script::{ScriptEngine, ScriptError, ScriptLanguage};
use spark_vm::{HostHooks, StdHost};

use crate::api::install_builtins;
use crate::loader::discover_and_order;

/// 引擎壳结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum EngineError {
    Spark(SparkError),
    Script(ScriptError),
    Plugin(PluginError),
    ModNotFound { id: String },
    MissingDep { mod_id: String, dep: String },
    CyclicDeps { mods: String },
    DuplicateMod { id: String },
    ManifestParse {
        path: String,
        source: crate::manifest::ManifestParseError,
    },
    ManifestMissingId { path: String },
    Io { path: String, detail: String },
    HookFailed {
        hook: String,
        mod_id: String,
        function: String,
        source: ScriptError,
    },
}

impl EngineError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Spark(e) => {
                // 透传 SparkError 的码字符串不便为 &'static；统一用包装码。
                let _ = e;
                "spark.engine.spark"
            }
            Self::Script(_) => "spark.engine.script",
            Self::Plugin(_) => "spark.engine.plugin",
            Self::ModNotFound { .. } => "spark.engine.mod_not_found",
            Self::MissingDep { .. } => "spark.engine.missing_dep",
            Self::CyclicDeps { .. } => "spark.engine.cyclic_deps",
            Self::DuplicateMod { .. } => "spark.engine.duplicate_mod",
            Self::ManifestParse { source, .. } => source.code(),
            Self::ManifestMissingId { .. } => "spark.engine.manifest_missing_id",
            Self::Io { .. } => "spark.engine.io",
            Self::HookFailed { .. } => "spark.engine.hook_failed",
        }
    }

    pub fn args(&self) -> spark_diagnostics::ErrorArgs {
        use spark_diagnostics::{ErrorArg, ErrorArgs};
        use std::sync::Arc;
        match self {
            Self::ModNotFound { id } | Self::DuplicateMod { id } => {
                ErrorArgs::new().with("id", ErrorArg::String(Arc::from(id.as_str())))
            }
            Self::MissingDep { mod_id, dep } => ErrorArgs::new()
                .with("mod_id", ErrorArg::String(Arc::from(mod_id.as_str())))
                .with("dep", ErrorArg::String(Arc::from(dep.as_str()))),
            Self::CyclicDeps { mods } => {
                ErrorArgs::new().with("mods", ErrorArg::String(Arc::from(mods.as_str())))
            }
            Self::ManifestParse { path, source } => source
                .args()
                .with("path", ErrorArg::Path(Arc::from(path.as_str()))),
            Self::ManifestMissingId { path } => {
                ErrorArgs::new().with("path", ErrorArg::Path(Arc::from(path.as_str())))
            }
            Self::Io { path, detail } => ErrorArgs::new()
                .with("path", ErrorArg::Path(Arc::from(path.as_str())))
                .with("opaque", ErrorArg::String(Arc::from(detail.as_str()))),
            Self::HookFailed {
                hook,
                mod_id,
                function,
                ..
            } => ErrorArgs::new()
                .with("hook", ErrorArg::String(Arc::from(hook.as_str())))
                .with("mod_id", ErrorArg::String(Arc::from(mod_id.as_str())))
                .with("function", ErrorArg::String(Arc::from(function.as_str()))),
            Self::Spark(_) | Self::Script(_) | Self::Plugin(_) => ErrorArgs::new(),
        }
    }

    pub fn io(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Io {
            path: path.into(),
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spark(e) => e.fmt(f),
            Self::Script(e) => e.fmt(f),
            Self::Plugin(e) => e.fmt(f),
            other => f.write_str(other.code()),
        }
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spark(e) => Some(e),
            Self::Script(e) => Some(e),
            Self::Plugin(e) => Some(e),
            Self::ManifestParse { source, .. } => Some(source),
            Self::HookFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<SparkError> for EngineError {
    fn from(value: SparkError) -> Self {
        Self::Spark(value)
    }
}

impl From<ScriptError> for EngineError {
    fn from(value: ScriptError) -> Self {
        Self::Script(value)
    }
}

impl From<PluginError> for EngineError {
    fn from(value: PluginError) -> Self {
        Self::Plugin(value)
    }
}

impl From<EngineError> for SparkError {
    fn from(e: EngineError) -> Self {
        match e {
            EngineError::Spark(s) => s,
            other => SparkError::new(spark_core::ErrorCode::parse(other.code())),
        }
    }
}

/// 模组间共享状态（钩子、数据表、日志缓冲、事件与本地化）。
#[derive(Default)]
pub struct EngineShared {
    pub hooks: HookBus,
    pub registry: DataRegistry,
    /// 脚本 `log` 原生写入，便于测试与宿主读取。
    pub logs: Vec<String>,
    pub events: spark_event::EventBus,
    pub localization: LocalizationService,
}

impl EngineShared {
    /// 帧边界：提交待切换 Locale，并翻转事件双缓冲。
    pub fn begin_frame(&mut self) -> Option<spark_localization::LocaleChanged> {
        let changed = self.localization.commit_pending(&mut self.events);
        self.events.update_all();
        changed
    }
}

/// VM 之上的引擎壳。
pub struct SparkEngine {
    shared: Rc<RefCell<EngineShared>>,
    mods: HashMap<String, LoadedMod>,
    mods_root: PathBuf,
    /// 脚本插件（Live2D 等）；Rust 宿主能力请直接 path 依赖 crate，勿塞这里。
    plugins: PluginRegistry,
}

impl SparkEngine {
    pub fn new(mods_root: impl Into<PathBuf>) -> Self {
        Self {
            shared: Rc::new(RefCell::new(EngineShared::default())),
            mods: HashMap::new(),
            mods_root: mods_root.into(),
            plugins: PluginRegistry::new(),
        }
    }

    pub fn plugins(&self) -> &PluginRegistry {
        &self.plugins
    }

    pub fn plugins_mut(&mut self) -> &mut PluginRegistry {
        &mut self.plugins
    }

    /// 注册脚本插件（须在 `load_*` 之前，以便编译期声明原生名）。
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<(), EngineError> {
        self.plugins.register(plugin)?;
        Ok(())
    }

    pub fn mods_root(&self) -> &Path {
        &self.mods_root
    }

    pub fn shared(&self) -> &Rc<RefCell<EngineShared>> {
        &self.shared
    }

    /// 帧边界：提交待切换 Locale 并翻转事件双缓冲。
    pub fn begin_frame(&self) -> Option<spark_localization::LocaleChanged> {
        self.shared.borrow_mut().begin_frame()
    }

    pub fn mod_ids(&self) -> impl Iterator<Item = &str> {
        self.mods.keys().map(|s| s.as_str())
    }

    pub fn get_mod(&self, id: &str) -> Option<&LoadedMod> {
        self.mods.get(id)
    }

    pub fn get_mod_mut(&mut self, id: &str) -> Option<&mut LoadedMod> {
        self.mods.get_mut(id)
    }

    /// 扫描 `mods_root` 下子目录，按依赖序加载全部模组。
    pub fn load_all(&mut self) -> Result<Vec<String>, EngineError> {
        let ordered = discover_and_order(&self.mods_root)?;
        let mut loaded = Vec::new();
        for manifest in ordered {
            let id = manifest.id.clone();
            self.load_manifest(manifest)?;
            loaded.push(id);
        }
        Ok(loaded)
    }

    /// 加载单个模组目录（内含 `mod.von`）。
    pub fn load_mod_dir(&mut self, dir: impl AsRef<Path>) -> Result<String, EngineError> {
        let dir = dir.as_ref();
        let manifest = ModManifest::from_dir(dir)?;
        let id = manifest.id.clone();
        self.load_manifest_at(manifest, dir.to_path_buf())?;
        Ok(id)
    }

    fn load_manifest(&mut self, manifest: ModManifest) -> Result<(), EngineError> {
        let dir = find_dir_for_id(&self.mods_root, &manifest.id)?;
        self.load_manifest_at(manifest, dir)
    }

    fn load_manifest_at(
        &mut self,
        manifest: ModManifest,
        root: PathBuf,
    ) -> Result<(), EngineError> {
        for dep in &manifest.dependencies {
            if !self.mods.contains_key(dep) {
                return Err(EngineError::MissingDep {
                    mod_id: manifest.id.clone(),
                    dep: dep.clone(),
                });
            }
        }

        let vfs = ModVfs::new(manifest.id.clone(), root.clone());
        let mut script = None;

        if let Some(entry) = &manifest.entry {
            let entry_path = root.join(entry);
            let source = std::fs::read_to_string(&entry_path).map_err(|e| {
                EngineError::io(entry_path.display().to_string(), e.to_string())
            })?;
            let lang = resolve_language(manifest.language.as_deref(), entry);
            let natives = self.compile_native_names();
            let mut eng = ScriptEngine::compile_with(lang, &source, &natives)?;
            install_builtins(&mut eng, &self.shared, &manifest.id, &vfs);
            self.plugins.install_all(&mut eng.vm);
            let mut host = StdHost;
            eng.eval_with(&mut host)?;
            script = Some(eng);
        }

        let id = manifest.id.clone();
        self.mods.insert(
            id.clone(),
            LoadedMod {
                manifest,
                root,
                vfs,
                script,
                enabled: true,
            },
        );
        tracing::info!(mod_id = %id, "模组已加载");
        Ok(())
    }

    /// 触发命名钩子：调用各模组已注册的脚本函数。
    pub fn fire_hook(
        &mut self,
        hook: &str,
        args: &[Value],
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        let refs: Vec<HookRef> = self.shared.borrow().hooks.list(hook).to_vec();
        for href in refs {
            if !self
                .mods
                .get(&href.mod_id)
                .map(|m| m.enabled)
                .unwrap_or(false)
            {
                continue;
            }
            let Some(m) = self.mods.get_mut(&href.mod_id) else {
                continue;
            };
            let Some(script) = m.script.as_mut() else {
                continue;
            };
            script.call(&href.function, args, host).map_err(|source| {
                EngineError::HookFailed {
                    hook: hook.to_string(),
                    mod_id: href.mod_id.clone(),
                    function: href.function.clone(),
                    source,
                }
            })?;
        }
        Ok(())
    }

    pub fn fire_hook_std(&mut self, hook: &str, args: &[Value]) -> Result<(), EngineError> {
        let mut host = StdHost;
        self.fire_hook(hook, args, &mut host)
    }

    /// 热重载：重新编译入口并保留已启用状态。
    pub fn reload_mod(&mut self, id: &str) -> Result<(), EngineError> {
        let (manifest, root, enabled) = {
            let m = self
                .mods
                .get(id)
                .ok_or_else(|| EngineError::ModNotFound { id: id.into() })?;
            (m.manifest.clone(), m.root.clone(), m.enabled)
        };
        self.shared.borrow_mut().hooks.remove_mod(id);
        self.mods.remove(id);
        self.load_manifest_at(manifest, root)?;
        if let Some(m) = self.mods.get_mut(id) {
            m.enabled = enabled;
        }
        Ok(())
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), EngineError> {
        let m = self
            .mods
            .get_mut(id)
            .ok_or_else(|| EngineError::ModNotFound { id: id.into() })?;
        m.enabled = enabled;
        Ok(())
    }

    pub fn resolve_asset(&self, mod_id: &str, rel: &str) -> Result<PathBuf, EngineError> {
        let m = self
            .mods
            .get(mod_id)
            .ok_or_else(|| EngineError::ModNotFound {
                id: mod_id.into(),
            })?;
        m.vfs.resolve(rel).map_err(EngineError::from)
    }

    fn compile_native_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = ENGINE_NATIVES.to_vec();
        for n in self.plugins.native_names() {
            if !names.iter().any(|x| *x == n) {
                names.push(n);
            }
        }
        names
    }
}

fn find_dir_for_id(root: &Path, id: &str) -> Result<PathBuf, EngineError> {
    let candidate = root.join(id);
    if candidate.join("mod.von").is_file() {
        return Ok(candidate);
    }
    let rd = std::fs::read_dir(root).map_err(|e| {
        EngineError::io(root.display().to_string(), e.to_string())
    })?;
    for ent in rd.flatten() {
        let p = ent.path();
        if !p.is_dir() {
            continue;
        }
        let von = p.join("mod.von");
        if !von.is_file() {
            continue;
        }
        if let Ok(m) = ModManifest::from_path(&von) {
            if m.id == id {
                return Ok(p);
            }
        }
    }
    Err(EngineError::ModNotFound { id: id.into() })
}

fn resolve_language(explicit: Option<&str>, entry: &str) -> ScriptLanguage {
    if let Some(s) = explicit {
        match s.to_ascii_lowercase().as_str() {
            "lua" => return ScriptLanguage::Lua,
            "ruby" | "rgss" => return ScriptLanguage::Ruby,
            "valkyrie" | "vk" | "v" => return ScriptLanguage::Valkyrie,
            _ => {}
        }
    }
    let ext = Path::new(entry)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "lua" => ScriptLanguage::Lua,
        "rb" | "rgss" => ScriptLanguage::Ruby,
        _ => ScriptLanguage::Valkyrie,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_vm::{FuncProto, Module, Op};

    #[test]
    fn registry_and_hooks_shared() {
        let eng = SparkEngine::new(".");
        {
            let mut s = eng.shared.borrow_mut();
            s.registry
                .set("meta", "version", RegValue::Number(1.0));
            s.hooks.register("init", "demo", "on_init");
        }
        assert_eq!(
            eng.shared
                .borrow()
                .registry
                .get("meta", "version")
                .and_then(|v| v.as_number()),
            Some(1.0)
        );
        assert_eq!(eng.shared.borrow().hooks.list("init").len(), 1);
    }

    #[test]
    fn manifest_roundtrip_von() {
        let raw = r#"
id = "demo"
name = "Demo"
version = "0.1.0"
entry = "main.vk"
dependencies = ["core"]
"#;
        let m = crate::manifest::parse_mod_von(raw).unwrap();
        assert_eq!(m.id, "demo");
        assert_eq!(m.dependencies, vec!["core"]);
    }

    #[test]
    fn vfs_stays_inside_root() {
        let dir = std::env::temp_dir().join("spark_engine_vfs_test");
        let _ = std::fs::create_dir_all(&dir);
        let vfs = ModVfs::new("t", dir.clone());
        let ok = vfs.resolve("a/b.txt").unwrap();
        assert!(ok.starts_with(&dir));
        assert!(vfs.resolve("../x").is_err());
    }

    #[test]
    fn hook_calls_script_function() {
        let mut f = FuncProto::new("on_init", 0);
        f.emit(Op::LoadNull);
        f.emit(Op::Return);
        let module = Module {
            functions: vec![f],
            entry: 0,
            native_names: Vec::new(),
        };
        let mut eng = SparkEngine::new(".");
        eng.shared
            .borrow_mut()
            .hooks
            .register("init", "hand", "on_init");
        eng.mods.insert(
            "hand".into(),
            LoadedMod {
                manifest: ModManifest {
                    id: "hand".into(),
                    name: "Hand".into(),
                    version: "0.0.1".into(),
                    entry: None,
                    language: None,
                    dependencies: vec![],
                },
                root: PathBuf::from("."),
                vfs: ModVfs::new("hand", PathBuf::from(".")),
                script: Some(ScriptEngine::from_module(module)),
                enabled: true,
            },
        );
        eng.fire_hook_std("init", &[]).unwrap();
    }

    #[test]
    fn topo_deps_order() {
        let root = std::env::temp_dir().join("spark_engine_mods_topo");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("base")).unwrap();
        std::fs::create_dir_all(root.join("child")).unwrap();
        std::fs::write(
            root.join("base/mod.von"),
            r#"id = "base"
version = "1.0.0"
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("child/mod.von"),
            r#"id = "child"
version = "1.0.0"
dependencies = ["base"]
"#,
        )
        .unwrap();
        let ordered = discover_and_order(&root).unwrap();
        assert_eq!(ordered[0].id, "base");
        assert_eq!(ordered[1].id, "child");
    }
}
