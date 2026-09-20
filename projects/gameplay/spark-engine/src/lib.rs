//! Spark 引擎壳：帧主循环编排 + ECS 宿主桥 + 在 `spark-vm` / `spark-script` 之上的 **modder** 能力。
//!
//! 提供：固定步 / update·draw 相位编排、[`EcsHost3d`]（`Schedule` ↔ `GameHost3d`）、
//! 模组清单与发现、依赖排序加载、脚本入口、命名钩子、
//! 通用数据表、模组资源路径、脚本插件挂载（[`PluginRegistry`]）。模组逻辑一律跑在
//! [`ScriptDomain`]（经 [`spark_script`] 编译为映像后装载），与宿主目标平台无关。
//! **不**拥有窗口后端（winit 等止于 `spark-renderer-wgpu` / 绑定宿主）。
//! **不**提供游戏内容权威（方块 / 配方等由游戏仓解释 [`DataRegistry`]）。
//! Rust 宿主若直接需要能力，请 path 依赖对应 crate，勿把 Rust API 伪装成插件。

mod api;
mod command_apply;
mod command_buffer;
mod domain;
mod ecs_host;
mod event_inbox;
mod frame;
mod hooks;
mod loader;
mod localization;
mod manifest;
mod query_view;
mod registry;
mod run;
mod script_system;
mod vfs;

pub use api::{BuiltinApi, ENGINE_NATIVES};
pub use command_apply::{
    apply_script_commands, apply_script_commands_with, CommandApplyReport, ComponentDescriptorId,
    ScriptArchetypeTag, ScriptComponentCatalog, ScriptMarker, SCRIPT_MARKER_NAME,
};
pub use command_buffer::{ScriptCommand, ScriptCommandBuffer};
pub use domain::{ScriptBudget, ScriptDomain};
pub use ecs_host::{DrawBuffer3d, EcsHost3d, FrameSnapshot};
pub use event_inbox::{ScriptEvent, ScriptEventInbox};
pub use frame::{
    FrameLoop, FrameLoopConfig, LoopedHost2d, LoopedHost3d, StepMode,
};
pub use hooks::{HookBus, HookRef};
pub use loader::{LoadedMod, ModLoader};
pub use localization::LocalizationService;
pub use manifest::{ManifestParseError, ModManifest};
pub use query_view::ScriptQueryView;
pub use registry::{DataRegistry, RegValue};
pub use run::{run_ecs_game_3d, run_game, run_game_3d, run_game_3d_with, run_game_with};
pub use script_system::{
    ComponentAccess, ScriptParallelism, ScriptSystemDescriptor, ScriptSystemRegistry,
};
pub use spark_plugin::{Plugin, PluginError, PluginInfo, PluginRegistry};
pub use vfs::ModVfs;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use spark_core::SparkError;
use spark_gc::Value;
use spark_script::{
    HostFunction, HostFunctionId, HostPhase, HostSchema, ScriptCompiler, ScriptError,
    ScriptLanguage,
};
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
    /// 文件系统失败：`kind` 为稳定机器令牌（如 `not_found`），不是 OS 本地化句子。
    Io { path: String, kind: String },
    HookFailed {
        hook: String,
        mod_id: String,
        function: String,
        source: ScriptError,
    },
    /// 脚本领域已被禁用（trap / 预算等）。
    ScriptDomainDisabled { mod_id: String },
}

impl EngineError {
    pub fn code(&self) -> String {
        match self {
            Self::Spark(e) => e.code.to_string(),
            Self::Script(e) => e.code().to_string(),
            Self::Plugin(e) => e.code().to_string(),
            Self::ModNotFound { .. } => "spark.engine.mod_not_found".into(),
            Self::MissingDep { .. } => "spark.engine.missing_dep".into(),
            Self::CyclicDeps { .. } => "spark.engine.cyclic_deps".into(),
            Self::DuplicateMod { .. } => "spark.engine.duplicate_mod".into(),
            Self::ManifestParse { source, .. } => source.code().to_string(),
            Self::ManifestMissingId { .. } => "spark.engine.manifest_missing_id".into(),
            Self::Io { .. } => "spark.engine.io".into(),
            Self::HookFailed { .. } => "spark.engine.hook_failed".into(),
            Self::ScriptDomainDisabled { .. } => "spark.engine.script_domain_disabled".into(),
        }
    }

    pub fn args(&self) -> spark_diagnostics::ErrorArgs {
        use spark_diagnostics::{ErrorArg, ErrorArgs};
        use std::sync::Arc;
        match self {
            Self::Spark(e) => e.args.clone(),
            Self::Script(e) => e.args(),
            Self::Plugin(e) => e.args(),
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
            Self::Io { path, kind } => ErrorArgs::new()
                .with("path", ErrorArg::Path(Arc::from(path.as_str())))
                .with("kind", ErrorArg::String(Arc::from(kind.as_str()))),
            Self::HookFailed {
                hook,
                mod_id,
                function,
                source,
            } => source.args()
                .with("hook", ErrorArg::String(Arc::from(hook.as_str())))
                .with("mod_id", ErrorArg::String(Arc::from(mod_id.as_str())))
                .with("function", ErrorArg::String(Arc::from(function.as_str()))),
            Self::ScriptDomainDisabled { mod_id } => ErrorArgs::new()
                .with("mod_id", ErrorArg::String(Arc::from(mod_id.as_str()))),
        }
    }

    /// 由 `std::io::Error` 构造；只保留稳定 `ErrorKind` 令牌。
    pub fn from_io(path: impl Into<String>, err: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            kind: io_kind_token(err.kind()).into(),
        }
    }
}

fn io_kind_token(kind: std::io::ErrorKind) -> &'static str {
    use std::io::ErrorKind::*;
    match kind {
        NotFound => "not_found",
        PermissionDenied => "permission_denied",
        ConnectionRefused => "connection_refused",
        ConnectionReset => "connection_reset",
        ConnectionAborted => "connection_aborted",
        NotConnected => "not_connected",
        AddrInUse => "addr_in_use",
        AddrNotAvailable => "addr_not_available",
        BrokenPipe => "broken_pipe",
        AlreadyExists => "already_exists",
        WouldBlock => "would_block",
        InvalidInput => "invalid_input",
        InvalidData => "invalid_data",
        TimedOut => "timed_out",
        WriteZero => "write_zero",
        Interrupted => "interrupted",
        Unsupported => "unsupported",
        UnexpectedEof => "unexpected_eof",
        OutOfMemory => "out_of_memory",
        _ => "other",
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spark(e) => e.fmt(f),
            Self::Script(e) => e.fmt(f),
            Self::Plugin(e) => e.fmt(f),
            other => f.write_str(&other.code()),
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
            other => {
                let code = spark_core::ErrorCode::parse(&other.code());
                let args = other.args();
                SparkError::new(code).with_args(args)
            }
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
    /// 已登记的脚本 System 描述符。
    script_systems: ScriptSystemRegistry,
}

impl SparkEngine {
    pub fn new(mods_root: impl Into<PathBuf>) -> Self {
        Self {
            shared: Rc::new(RefCell::new(EngineShared::default())),
            mods: HashMap::new(),
            mods_root: mods_root.into(),
            plugins: PluginRegistry::new(),
            script_systems: ScriptSystemRegistry::new(),
        }
    }

    pub fn plugins(&self) -> &PluginRegistry {
        &self.plugins
    }

    pub fn plugins_mut(&mut self) -> &mut PluginRegistry {
        &mut self.plugins
    }

    pub fn script_systems(&self) -> &ScriptSystemRegistry {
        &self.script_systems
    }

    pub fn script_systems_mut(&mut self) -> &mut ScriptSystemRegistry {
        &mut self.script_systems
    }

    /// 登记脚本 System 描述符（同 `mod_id`+`name` 覆盖）。
    pub fn register_script_system(&mut self, desc: ScriptSystemDescriptor) {
        self.script_systems.register(desc);
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
        let mut domain = None;

        if let Some(entry) = &manifest.entry {
            let entry_path = root.join(entry);
            let source = std::fs::read_to_string(&entry_path).map_err(|e| {
                EngineError::from_io(entry_path.display().to_string(), e)
            })?;
            let lang = resolve_language(manifest.language.as_deref(), entry);
            let natives = self.compile_native_names();
            let host_schema = host_schema_from_names(&natives);
            // 编译与装载必须共用同一份 schema（哈希校验）。
            let package = ScriptCompiler::new()
                .compile_source(lang, &source, &host_schema)
                .map_err(EngineError::Script)?;
            let mut script_domain = ScriptDomain::from_image(
                manifest.id.as_str(),
                &package.image,
                &host_schema,
                ScriptBudget::default(),
            )?;
            install_builtins(
                &mut script_domain.runtime.vm,
                &self.shared,
                &manifest.id,
                &vfs,
                &script_domain.command_buffer,
            );
            self.plugins.install_all(&mut script_domain.runtime.vm);
            let mut hooks = StdHost;
            // 有 `on_load` 则走生命周期；否则过渡期仍执行顶层（`register_hook` 等）。
            if script_domain.has_lifecycle("on_load") {
                let _ = script_domain.call_lifecycle("on_load", &[], &mut hooks)?;
            } else {
                script_domain.eval_entry(&mut hooks)?;
            }
            self.script_systems.register_lifecycle_exports(
                manifest.id.as_str(),
                &script_domain.lifecycle_exports,
            );
            domain = Some(script_domain);
        }

        let id = manifest.id.clone();
        self.mods.insert(
            id.clone(),
            LoadedMod {
                manifest,
                root,
                vfs,
                domain,
                enabled: true,
            },
        );
        tracing::info!(event = "spark.engine.mod_loaded", mod_id = %id);
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
            let Some(domain) = m.domain.as_mut() else {
                continue;
            };
            domain
                .call(&href.function, args, host)
                .map_err(|err| match err {
                    EngineError::Script(source) => EngineError::HookFailed {
                        hook: hook.to_string(),
                        mod_id: href.mod_id.clone(),
                        function: href.function.clone(),
                        source,
                    },
                    other => other,
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
        self.script_systems.remove_mod(id);
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

    /// 帧同步点：按模组加载顺序取出并清空各领域命令缓冲。
    pub fn drain_script_commands(&mut self) -> Vec<(String, Vec<ScriptCommand>)> {
        let mut out = Vec::new();
        for (id, m) in &mut self.mods {
            if let Some(domain) = m.domain.as_mut() {
                let cmds = domain.drain_commands();
                if !cmds.is_empty() {
                    out.push((id.clone(), cmds));
                }
            }
        }
        out
    }

    /// 取出各领域命令并提交到 ECS [`spark_ecs::World`]。
    pub fn apply_script_commands_to_world(
        &mut self,
        world: &mut spark_ecs::World,
    ) -> CommandApplyReport {
        let batches = self.drain_script_commands();
        let mut report = CommandApplyReport::default();
        for (_mod_id, cmds) in batches {
            report.merge(apply_script_commands(world, &cmds));
        }
        report
    }

    /// 向指定模组领域入队事件（不立即派发）。
    pub fn enqueue_script_event(
        &mut self,
        mod_id: &str,
        name: impl Into<std::sync::Arc<str>>,
        args: Vec<Value>,
    ) -> Result<(), EngineError> {
        let m = self
            .mods
            .get_mut(mod_id)
            .ok_or_else(|| EngineError::ModNotFound {
                id: mod_id.into(),
            })?;
        let domain = m
            .domain
            .as_mut()
            .ok_or_else(|| EngineError::ModNotFound {
                id: mod_id.into(),
            })?;
        domain.enqueue_event(name, args);
        Ok(())
    }

    /// 派发所有已启用领域的事件 inbox。
    pub fn dispatch_script_events(
        &mut self,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        let ids: Vec<String> = self.mods.keys().cloned().collect();
        for id in ids {
            let Some(m) = self.mods.get_mut(&id) else {
                continue;
            };
            if !m.enabled {
                continue;
            }
            let Some(domain) = m.domain.as_mut() else {
                continue;
            };
            domain.dispatch_events(host)?;
        }
        Ok(())
    }

    /// 驱动脚本领域一帧：生命周期导出 → 事件派发 → 取出命令缓冲。
    ///
    /// `fixed` 为 true 时优先调用 `fixed_update`，否则调用 `update`。
    /// 命令缓冲仅收集返回，由宿主在同步点提交到 ECS（本层不拥有 `World`）。
    pub fn tick_scripts(
        &mut self,
        fixed: bool,
        host: &mut dyn HostHooks,
    ) -> Result<Vec<(String, Vec<ScriptCommand>)>, EngineError> {
        let phase = if fixed {
            HostPhase::FixedUpdate
        } else {
            HostPhase::Update
        };
        self.run_script_phase(phase, host)?;
        self.dispatch_script_events(host)?;
        Ok(self.drain_script_commands())
    }

    /// 按 [`HostPhase`] 调度已登记的脚本 System（同域串行）。
    ///
    /// 每个描述符调用其 `entry` 导出；调用后不自动提交命令（由宿主调用
    /// [`Self::apply_script_commands_to_world`]）。
    pub fn run_script_phase(
        &mut self,
        phase: HostPhase,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        let jobs: Vec<(String, String)> = self
            .script_systems
            .for_phase(phase)
            .map(|s| (s.mod_id.to_string(), s.entry.to_string()))
            .collect();
        for (mod_id, entry) in jobs {
            let Some(m) = self.mods.get_mut(&mod_id) else {
                continue;
            };
            if !m.enabled {
                continue;
            }
            let Some(domain) = m.domain.as_mut() else {
                continue;
            };
            if !domain.enabled {
                continue;
            }
            let has_entry = domain
                .runtime
                .vm
                .module
                .functions
                .iter()
                .any(|f| f.name == entry);
            if has_entry {
                let _ = domain.call(&entry, &[], host)?;
            }
        }
        Ok(())
    }

    /// 调度某一 phase 的脚本 System，派发事件，并把命令提交到 `world`。
    pub fn run_script_systems(
        &mut self,
        phase: HostPhase,
        world: &mut spark_ecs::World,
        host: &mut dyn HostHooks,
    ) -> Result<CommandApplyReport, EngineError> {
        self.run_script_phase(phase, host)?;
        self.dispatch_script_events(host)?;
        Ok(self.apply_script_commands_to_world(world))
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

fn host_schema_from_names(names: &[&str]) -> HostSchema {
    let mut schema = HostSchema::new(1);
    for name in names {
        schema.insert(HostFunction::new(HostFunctionId::new(
            "spark.engine",
            *name,
            1,
        )));
    }
    schema
}

fn find_dir_for_id(root: &Path, id: &str) -> Result<PathBuf, EngineError> {
    let candidate = root.join(id);
    if candidate.join("mod.von").is_file() {
        return Ok(candidate);
    }
    let rd = std::fs::read_dir(root).map_err(|e| {
        EngineError::from_io(root.display().to_string(), e)
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
                domain: Some(ScriptDomain::from_legacy_module(
                    "hand",
                    module,
                    ScriptLanguage::Valkyrie,
                    ScriptBudget::default(),
                )),
                enabled: true,
            },
        );
        eng.fire_hook_std("init", &[]).unwrap();
    }

    #[test]
    fn load_mod_dir_shares_host_schema() {
        let root = std::env::temp_dir().join("spark_engine_mod_schema");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("mod.von"),
            r#"id = "schema_demo"
version = "0.1.0"
entry = "main.vk"
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("main.vk"),
            r#"
            micro on_load() {
                return 1
            }
            return 0
            "#,
        )
        .unwrap();
        let mut eng = SparkEngine::new(root.parent().unwrap());
        let id = eng.load_mod_dir(&root).unwrap();
        assert_eq!(id, "schema_demo");
        let m = eng.get_mod(&id).unwrap();
        let domain = m.domain.as_ref().unwrap();
        assert!(domain.enabled);
        assert!(domain.has_lifecycle("on_load"));
        assert_eq!(
            domain.runtime.vm.step_limit,
            ScriptBudget::default().instruction_limit
        );
    }

    #[test]
    fn load_mod_registers_lifecycle_systems() {
        let root = std::env::temp_dir().join("spark_engine_mod_systems");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("mod.von"),
            r#"id = "sys_demo"
version = "0.1.0"
entry = "main.vk"
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("main.vk"),
            r#"
            micro on_load() {
                return 1
            }
            micro update() {
                return 2
            }
            return 0
            "#,
        )
        .unwrap();
        let mut eng = SparkEngine::new(root.parent().unwrap());
        eng.load_mod_dir(&root).unwrap();
        assert!(eng.script_systems().len() >= 2);
        assert!(eng
            .script_systems()
            .for_phase(spark_script::HostPhase::Update)
            .any(|s| s.mod_id.as_ref() == "sys_demo"));
    }

    #[test]
    fn apply_script_commands_spawns_in_world() {
        let root = std::env::temp_dir().join("spark_engine_mod_apply");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("mod.von"),
            r#"id = "apply_demo"
version = "0.1.0"
entry = "main.vk"
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("main.vk"),
            r#"
            micro on_load() {
                queue_spawn("rock")
                return 1
            }
            return 0
            "#,
        )
        .unwrap();
        let mut eng = SparkEngine::new(root.parent().unwrap());
        eng.load_mod_dir(&root).unwrap();
        let mut world = spark_ecs::World::new();
        let report = eng.apply_script_commands_to_world(&mut world);
        assert_eq!(report.spawned.len(), 1);
        let e = report.spawned[0];
        assert_eq!(
            world
                .get::<ScriptArchetypeTag>(e)
                .map(|t| t.name.as_ref()),
            Some("rock")
        );
    }

    #[test]
    fn run_script_systems_applies_spawn_from_update() {
        let root = std::env::temp_dir().join("spark_engine_mod_run_sys");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("mod.von"),
            r#"id = "run_sys"
version = "0.1.0"
entry = "main.vk"
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("main.vk"),
            r#"
            micro on_load() {
                return 0
            }
            micro update() {
                queue_spawn("npc")
                return 1
            }
            return 0
            "#,
        )
        .unwrap();
        let mut eng = SparkEngine::new(root.parent().unwrap());
        eng.load_mod_dir(&root).unwrap();
        let mut world = spark_ecs::World::new();
        let mut hooks = StdHost;
        let report = eng
            .run_script_systems(HostPhase::Update, &mut world, &mut hooks)
            .unwrap();
        assert_eq!(report.spawned.len(), 1);
        let view = ScriptQueryView::new(&world);
        assert_eq!(view.entities_with_archetype("npc").len(), 1);
    }

    #[test]
    fn tick_scripts_calls_update_lifecycle() {
        let source = r#"
            micro update() {
                return 9
            }
            return 0
            "#;
        let host = HostSchema::new(1);
        let mut compiler = ScriptCompiler::new();
        let package = compiler
            .compile_source(ScriptLanguage::Valkyrie, source, &host)
            .unwrap();
        let domain = ScriptDomain::from_image(
            "tick.mod",
            &package.image,
            &host,
            ScriptBudget::default(),
        )
        .unwrap();
        domain.command_buffer.borrow_mut().spawn("marker");
        let mut eng = SparkEngine::new(".");
        eng.mods.insert(
            "tick.mod".into(),
            LoadedMod {
                manifest: ModManifest {
                    id: "tick.mod".into(),
                    name: "Tick".into(),
                    version: "0.0.1".into(),
                    entry: None,
                    language: None,
                    dependencies: vec![],
                },
                root: PathBuf::from("."),
                vfs: ModVfs::new("tick.mod", PathBuf::from(".")),
                domain: Some(domain),
                enabled: true,
            },
        );
        let mut hooks = StdHost;
        let cmds = eng.tick_scripts(false, &mut hooks).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].0, "tick.mod");
        assert_eq!(cmds[0].1.len(), 1);
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
