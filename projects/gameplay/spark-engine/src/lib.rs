//! Spark 引擎壳：帧主循环编排 + ECS 宿主桥 + 在 `spark-vm` / `spark-script` 之上的 **modder** 能力。
//!
//! 提供：固定步 / update·draw 相位编排、[`EcsHost2d`] / [`EcsHost3d`]（`Schedule` ↔ `GameHost`）、
//! 模组清单与发现、依赖排序加载、脚本入口、命名钩子、
//! 通用数据表、模组资源路径、脚本插件挂载（[`PluginRegistry`]）。模组逻辑一律跑在
//! [`ScriptDomain`]（经 [`spark_script`] 编译为映像后装载），与宿主目标平台无关。
//! **不**拥有窗口后端（winit 等止于 `spark-renderer-wgpu` / 绑定宿主）。
//! **不**提供游戏内容权威（方块 / 配方等由游戏仓解释 [`DataRegistry`]）。
//! Rust 宿主若直接需要能力，请 path 依赖对应 crate，勿把 Rust API 伪装成插件。

#![forbid(missing_docs)]

pub mod access_policy;
pub mod api;
pub mod app;
pub mod app3d;
pub mod command_apply;
pub mod command_buffer;
pub mod domain;
pub mod ecs_host;
pub mod event_inbox;
pub mod frame;
pub mod hooks;
pub mod loader;
pub mod localization;
pub mod manifest;
pub mod query_view;
pub mod registry;
pub mod render2d;
pub mod render3d;
pub mod run;
pub mod script_system;
pub mod vfs;

pub use access_policy::{ScriptAccessPolicy, check_host_determinism, check_host_phase};
pub use api::{BuiltinApi, ENGINE_NATIVES, engine_host_schema};
pub use app::{SparkApp, SparkPlugin};
pub use app3d::{SparkApp3d, SparkPlugin3d};
pub use command_apply::{
    CommandApplyError, CommandApplyReport, ComponentDescriptorId, SCRIPT_MARKER_NAME, ScriptArchetypeTag, ScriptComponentCatalog, ScriptMarker,
    apply_script_commands, apply_script_commands_with,
};
pub use command_buffer::{ScriptCommand, ScriptCommandBuffer};
pub use domain::{ScriptBudget, ScriptDomain};
pub use ecs_host::{AppExit, DrawBuffer2d, DrawBuffer3d, DrawScratch2d, EcsHost2d, EcsHost3d, FrameSnapshot, OsCursorVisible};
pub use event_inbox::{ScriptEvent, ScriptEventInbox};
pub use frame::{FrameLoop, FrameLoopConfig, LoopedHost2d, LoopedHost3d, StepMode};
pub use hooks::{HookBus, HookRef};
pub use loader::{LoadedMod, ModLoader, discover_and_order};
pub use localization::LocalizationService;
pub use manifest::{ManifestParseError, ModManifest, parse_mod_von};
pub use query_view::{ScriptQuerySnapshot, ScriptQueryView};
pub use registry::{DataRegistry, RegValue};
pub use render2d::{RenderFrame2d, RenderSchedule2d, RenderSystem2d};
pub use render3d::{RenderFrame3d, RenderSchedule3d, RenderSystem3d};
pub use run::{run_app_2d, run_app_3d, run_ecs_game, run_ecs_game_2d, run_ecs_game_3d, run_game, run_game_3d, run_game_3d_with, run_game_with};
pub use script_system::{ComponentAccess, ScriptParallelism, ScriptSystemDescriptor, ScriptSystemError, ScriptSystemRegistry};
pub use spark_plugin::{Plugin, PluginError, PluginInfo, PluginRegistry};
pub use vfs::ModVfs;

use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use spark_gc::Value;
use spark_script::{
    ArtifactCache, CompilationRequest, DeterminismClass, ExecutableImage, HostFunction, HostFunctionId, HostPhase, HostSchema, PackageId,
    ScriptCompiler, ScriptError, ScriptLanguage,
};
use spark_types::SparkError;
use spark_vm::{HostHooks, StdHost};

use crate::api::install_builtins;

/// 引擎壳结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum EngineError {
    /// 下层 `spark-types` 错误透传。
    Spark(SparkError),
    /// 脚本编译 / 装载 / 运行时错误透传。
    Script(ScriptError),
    /// 脚本插件登记失败透传。
    Plugin(PluginError),
    /// 按 id 查找模组失败。
    ModNotFound {
        /// 缺失的模组 id。
        id: String,
    },
    /// 装载时声明了未发现的依赖。
    MissingDep {
        /// 声明依赖的模组 id。
        mod_id: String,
        /// 缺失的依赖 id。
        dep: String,
    },
    /// 依赖图存在环。
    CyclicDeps {
        /// 仍在环中的模组 id 列表（逗号拼接）。
        mods: String,
    },
    /// 同一 mods 根下出现重复模组 id。
    DuplicateMod {
        /// 重复的模组 id。
        id: String,
    },
    /// `mod.von` 语法/字段解析失败。
    ManifestParse {
        /// 清单文件路径。
        path: String,
        /// 具体解析错误。
        source: crate::manifest::ManifestParseError,
    },
    /// 清单缺少必填 `id`。
    ManifestMissingId {
        /// 清单文件路径。
        path: String,
    },
    /// 文件系统失败：`kind` 为稳定机器令牌（如 `not_found`），不是 OS 本地化句子。
    Io {
        /// 相关路径（展示用字符串）。
        path: String,
        /// 稳定 `ErrorKind` 令牌。
        kind: String,
    },
    /// 触发命名钩子时某模组导出调用失败。
    HookFailed {
        /// 钩子名。
        hook: String,
        /// 失败的模组 id。
        mod_id: String,
        /// 失败的导出函数名。
        function: String,
        /// 底层脚本错误。
        source: ScriptError,
    },
    /// 脚本领域已被禁用（trap / 预算等）。
    ScriptDomainDisabled {
        /// 被禁用领域所属模组 id。
        mod_id: String,
    },
    /// 脚本 System 声明 / 调度契约失败。
    ScriptSystem(ScriptSystemError),
    /// 脚本命令提交失败（未知 / 不支持组件等）。
    CommandApply(crate::command_apply::CommandApplyError),
    /// 模组语言无法解析（显式字段或入口扩展名）。
    UnknownLanguage {
        /// 无法识别的语言 token 或 `ext:...`。
        token: String,
    },
}

impl EngineError {
    /// 稳定错误码字符串（多数变体为 `spark.engine.*`）。
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
            Self::ScriptSystem(_) => "spark.engine.script_system".into(),
            Self::CommandApply(e) => e.code().into(),
            Self::UnknownLanguage { .. } => "spark.engine.unknown_language".into(),
        }
    }

    /// 结构化诊断参数（供本地化模板 / `SparkError` 转换使用）。
    pub fn args(&self) -> spark_diagnostics::ErrorArgs {
        use spark_diagnostics::{ErrorArg, ErrorArgs};
        use std::sync::Arc;
        match self {
            Self::Spark(e) => e.args.clone(),
            Self::Script(e) => e.args(),
            Self::Plugin(e) => e.args(),
            Self::ModNotFound { id } | Self::DuplicateMod { id } => ErrorArgs::new().with("id", ErrorArg::String(Arc::from(id.as_str()))),
            Self::MissingDep { mod_id, dep } => ErrorArgs::new()
                .with("mod_id", ErrorArg::String(Arc::from(mod_id.as_str())))
                .with("dep", ErrorArg::String(Arc::from(dep.as_str()))),
            Self::CyclicDeps { mods } => ErrorArgs::new().with("mods", ErrorArg::String(Arc::from(mods.as_str()))),
            Self::ManifestParse { path, source } => source.args().with("path", ErrorArg::Path(Arc::from(path.as_str()))),
            Self::ManifestMissingId { path } => ErrorArgs::new().with("path", ErrorArg::Path(Arc::from(path.as_str()))),
            Self::Io { path, kind } => {
                ErrorArgs::new().with("path", ErrorArg::Path(Arc::from(path.as_str()))).with("kind", ErrorArg::String(Arc::from(kind.as_str())))
            }
            Self::HookFailed { hook, mod_id, function, source } => source
                .args()
                .with("hook", ErrorArg::String(Arc::from(hook.as_str())))
                .with("mod_id", ErrorArg::String(Arc::from(mod_id.as_str())))
                .with("function", ErrorArg::String(Arc::from(function.as_str()))),
            Self::ScriptDomainDisabled { mod_id } => ErrorArgs::new().with("mod_id", ErrorArg::String(Arc::from(mod_id.as_str()))),
            Self::ScriptSystem(e) => ErrorArgs::new().with("detail", ErrorArg::String(Arc::from(e.to_string()))),
            Self::CommandApply(e) => match e {
                crate::command_apply::CommandApplyError::UnknownComponent { component }
                | crate::command_apply::CommandApplyError::UnsupportedComponent { component }
                | crate::command_apply::CommandApplyError::EntityNotAlive { component, .. } => {
                    ErrorArgs::new().with("component", ErrorArg::String(Arc::clone(component)))
                }
            },
            Self::UnknownLanguage { token } => ErrorArgs::new().with("token", ErrorArg::String(Arc::from(token.as_str()))),
        }
    }

    /// 由 `std::io::Error` 构造；只保留稳定 `ErrorKind` 令牌。
    pub fn from_io(path: impl Into<String>, err: std::io::Error) -> Self {
        Self::Io { path: path.into(), kind: io_kind_token(err.kind()).into() }
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

impl From<ScriptSystemError> for EngineError {
    fn from(value: ScriptSystemError) -> Self {
        Self::ScriptSystem(value)
    }
}

impl From<crate::command_apply::CommandApplyError> for EngineError {
    fn from(value: crate::command_apply::CommandApplyError) -> Self {
        Self::CommandApply(value)
    }
}

impl From<EngineError> for SparkError {
    fn from(e: EngineError) -> Self {
        match e {
            EngineError::Spark(s) => s,
            other => {
                let code = spark_types::ErrorCode::parse(&other.code());
                let args = other.args();
                SparkError::new(code).with_args(args)
            }
        }
    }
}

/// 模组间共享状态（钩子、数据表、日志缓冲、事件与本地化）。
#[derive(Default)]
pub struct EngineShared {
    /// 命名钩子总线（脚本注册，宿主 `fire_hook` 触发）。
    pub hooks: HookBus,
    /// 跨模组通用数据表。
    pub registry: DataRegistry,
    /// 脚本 `log` 原生写入，便于测试与宿主读取。
    pub logs: Vec<String>,
    /// 引擎侧事件总线（本地化变更等）。
    pub events: spark_event::EventBus,
    /// 帧边界提交的本地化服务。
    pub localization: LocalizationService,
    /// 帧同步点拍摄的全量 ECS 只读快照。
    pub query_base: ScriptQuerySnapshot,
    /// 当前脚本调用可见的查询视图（可能经原型过滤）。
    pub query: ScriptQuerySnapshot,
    /// 当前脚本调用的生命周期阶段（由调度器写入）。
    pub active_phase: HostPhase,
    /// 当前脚本调用的确定性要求。
    pub active_determinism: DeterminismClass,
    /// 当前脚本调用的组件访问策略。
    pub access: ScriptAccessPolicy,
    /// 与编译期一致的宿主 ABI（阶段 / 效果门禁）。
    pub host_schema: HostSchema,
}

impl EngineShared {
    /// 帧边界：提交待切换 Locale，并翻转事件双缓冲。
    pub fn begin_frame(&mut self) -> Option<spark_localization::LocaleChanged> {
        let changed = self.localization.commit_pending(&mut self.events);
        self.events.update_all();
        changed
    }

    /// 进入一次脚本导出调用前设置阶段、访问契约与受限查询视图。
    pub fn begin_script_call(&mut self, phase: HostPhase, desc: Option<&ScriptSystemDescriptor>) {
        self.active_phase = phase;
        self.active_determinism = desc.map(|d| d.determinism).unwrap_or(DeterminismClass::Nondeterministic);
        self.access = match desc {
            Some(d) => ScriptAccessPolicy::from_descriptor(d),
            None => ScriptAccessPolicy::Unrestricted,
        };
        self.install_query_view();
    }

    /// 按当前 `access` 从 `query_base` 安装可见查询快照。
    pub fn install_query_view(&mut self) {
        self.query = match self.access.archetype_filter() {
            Some(allow) => self.query_base.filtered_by_archetypes(allow),
            None => self.query_base.clone(),
        };
    }

    /// 调用结束后恢复为未声明阶段 + 无限制访问 + 全量查询视图。
    pub fn end_script_call(&mut self) {
        self.active_phase = HostPhase::Any;
        self.active_determinism = DeterminismClass::Nondeterministic;
        self.access = ScriptAccessPolicy::Unrestricted;
        self.query = self.query_base.clone();
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
    /// 以模组根目录构造空引擎壳（尚无装载模组；内置宿主 schema 已就绪）。
    pub fn new(mods_root: impl Into<PathBuf>) -> Self {
        let mut shared = EngineShared::default();
        shared.host_schema = crate::api::engine_host_schema();
        Self {
            shared: Rc::new(RefCell::new(shared)),
            mods: HashMap::new(),
            mods_root: mods_root.into(),
            plugins: PluginRegistry::new(),
            script_systems: ScriptSystemRegistry::new(),
        }
    }

    /// 只读访问脚本插件登记表。
    pub fn plugins(&self) -> &PluginRegistry {
        &self.plugins
    }

    /// 可变访问脚本插件登记表（须在 `load_*` 前 `register_plugin`）。
    pub fn plugins_mut(&mut self) -> &mut PluginRegistry {
        &mut self.plugins
    }

    /// 只读访问脚本 System 登记表。
    pub fn script_systems(&self) -> &ScriptSystemRegistry {
        &self.script_systems
    }

    /// 可变访问脚本 System 登记表。
    pub fn script_systems_mut(&mut self) -> &mut ScriptSystemRegistry {
        &mut self.script_systems
    }

    /// 登记脚本 System 描述符（同 `mod_id`+`name` 覆盖），并校验声明契约。
    pub fn register_script_system(&mut self, desc: ScriptSystemDescriptor) -> Result<(), EngineError> {
        self.script_systems.register_checked(desc)?;
        Ok(())
    }

    /// 注册脚本插件（须在 `load_*` 之前，以便编译期声明原生名）。
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<(), EngineError> {
        self.plugins.register(plugin)?;
        Ok(())
    }

    /// 模组扫描根目录。
    pub fn mods_root(&self) -> &Path {
        &self.mods_root
    }

    /// 共享状态句柄（内置原生与宿主侧共用）。
    pub fn shared(&self) -> &Rc<RefCell<EngineShared>> {
        &self.shared
    }

    /// 帧边界：提交待切换 Locale 并翻转事件双缓冲。
    pub fn begin_frame(&self) -> Option<spark_localization::LocaleChanged> {
        self.shared.borrow_mut().begin_frame()
    }

    /// 已装载模组 id 迭代器（无序）。
    pub fn mod_ids(&self) -> impl Iterator<Item = &str> {
        self.mods.keys().map(|s| s.as_str())
    }

    /// 按 id 取已装载模组。
    pub fn get_mod(&self, id: &str) -> Option<&LoadedMod> {
        self.mods.get(id)
    }

    /// 按 id 可变取已装载模组。
    pub fn get_mod_mut(&mut self, id: &str) -> Option<&mut LoadedMod> {
        self.mods.get_mut(id)
    }

    /// 直接挂入已构造的模组（测试 / 宿主自建映像）。
    pub fn insert_mod(&mut self, id: impl Into<String>, loaded: LoadedMod) {
        self.mods.insert(id.into(), loaded);
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

    fn load_manifest_at(&mut self, manifest: ModManifest, root: PathBuf) -> Result<(), EngineError> {
        for dep in &manifest.dependencies {
            if !self.mods.contains_key(dep) {
                return Err(EngineError::MissingDep { mod_id: manifest.id.clone(), dep: dep.clone() });
            }
        }

        let vfs = ModVfs::new(manifest.id.clone(), root.clone());
        let mut domain = None;

        if manifest.artifact.is_some() || manifest.entry.is_some() {
            let host_schema = self.build_host_schema();
            self.shared.borrow_mut().host_schema = host_schema.clone();
            let image = load_mod_image(&manifest, &root, &host_schema)?;
            let mut script_domain = ScriptDomain::from_image(manifest.id.as_str(), &image, &host_schema, ScriptBudget::default())?;
            install_builtins(&mut script_domain.runtime.vm, &self.shared, &manifest.id, &vfs, &script_domain.command_buffer);
            self.plugins.install_all(&mut script_domain.runtime.vm);
            let mut hooks = StdHost;
            // 装载只跑 `on_load`（顶层块已在封目标时提升为 `on_load`）。
            self.shared.borrow_mut().begin_script_call(HostPhase::OnLoad, None);
            let load_result = script_domain.call_lifecycle("on_load", &[], &mut hooks);
            self.shared.borrow_mut().end_script_call();
            let _ = load_result?;
            self.script_systems.register_lifecycle_exports(manifest.id.as_str(), &script_domain.lifecycle_exports);
            domain = Some(script_domain);
        }

        let id = manifest.id.clone();
        self.mods.insert(id.clone(), LoadedMod { manifest, root, vfs, domain, enabled: true });
        tracing::info!(event = "spark.engine.mod_loaded", mod_id = %id);
        Ok(())
    }

    /// 触发命名钩子：调用各模组已注册的脚本函数。
    pub fn fire_hook(&mut self, hook: &str, args: &[Value], host: &mut dyn HostHooks) -> Result<(), EngineError> {
        let refs: Vec<HookRef> = self.shared.borrow().hooks.list(hook).to_vec();
        for href in refs {
            if !self.mods.get(&href.mod_id).map(|m| m.enabled).unwrap_or(false) {
                continue;
            }
            let Some(m) = self.mods.get_mut(&href.mod_id)
            else {
                continue;
            };
            let Some(domain) = m.domain.as_mut()
            else {
                continue;
            };
            domain.call(&href.function, args, host).map_err(|err| match err {
                EngineError::Script(source) => {
                    EngineError::HookFailed { hook: hook.to_string(), mod_id: href.mod_id.clone(), function: href.function.clone(), source }
                }
                other => other,
            })?;
        }
        Ok(())
    }

    /// 使用默认 [`StdHost`] 触发命名钩子。
    pub fn fire_hook_std(&mut self, hook: &str, args: &[Value]) -> Result<(), EngineError> {
        let mut host = StdHost;
        self.fire_hook(hook, args, &mut host)
    }

    /// 热重载：重新编译入口并保留已启用状态。
    pub fn reload_mod(&mut self, id: &str) -> Result<(), EngineError> {
        let (manifest, root, enabled) = {
            let m = self.mods.get(id).ok_or_else(|| EngineError::ModNotFound { id: id.into() })?;
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

    /// 启用或禁用模组（禁用后跳过钩子与脚本调度）；未知 id → [`EngineError::ModNotFound`]。
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), EngineError> {
        let m = self.mods.get_mut(id).ok_or_else(|| EngineError::ModNotFound { id: id.into() })?;
        m.enabled = enabled;
        Ok(())
    }

    /// 在模组沙箱内解析相对资源路径；未知模组或路径逃逸均报错。
    pub fn resolve_asset(&self, mod_id: &str, rel: &str) -> Result<PathBuf, EngineError> {
        let m = self.mods.get(mod_id).ok_or_else(|| EngineError::ModNotFound { id: mod_id.into() })?;
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
    pub fn apply_script_commands_to_world(&mut self, world: &mut spark_ecs::World) -> Result<CommandApplyReport, EngineError> {
        let batches = self.drain_script_commands();
        let mut report = CommandApplyReport::default();
        for (_mod_id, cmds) in batches {
            report.merge(apply_script_commands(world, &cmds)?);
        }
        self.refresh_script_query(world);
        Ok(report)
    }

    /// 从当前世界刷新脚本可读查询快照（应在提交命令后、跑脚本前调用）。
    pub fn refresh_script_query(&mut self, world: &spark_ecs::World) {
        let mut shared = self.shared.borrow_mut();
        shared.query_base = ScriptQuerySnapshot::from_world(world);
        shared.install_query_view();
    }

    /// 向指定模组领域入队事件（不立即派发）。
    pub fn enqueue_script_event(&mut self, mod_id: &str, name: impl Into<std::sync::Arc<str>>, args: Vec<Value>) -> Result<(), EngineError> {
        let m = self.mods.get_mut(mod_id).ok_or_else(|| EngineError::ModNotFound { id: mod_id.into() })?;
        let domain = m.domain.as_mut().ok_or_else(|| EngineError::ModNotFound { id: mod_id.into() })?;
        domain.enqueue_event(name, args);
        Ok(())
    }

    /// 派发所有已启用领域的事件 inbox。
    pub fn dispatch_script_events(&mut self, host: &mut dyn HostHooks) -> Result<(), EngineError> {
        let ids: Vec<String> = self.mods.keys().cloned().collect();
        for id in ids {
            let Some(m) = self.mods.get_mut(&id)
            else {
                continue;
            };
            if !m.enabled {
                continue;
            }
            let Some(domain) = m.domain.as_mut()
            else {
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
    pub fn tick_scripts(&mut self, fixed: bool, host: &mut dyn HostHooks) -> Result<Vec<(String, Vec<ScriptCommand>)>, EngineError> {
        let phase = if fixed { HostPhase::FixedUpdate } else { HostPhase::Update };
        self.run_script_phase(phase, host)?;
        self.dispatch_script_events(host)?;
        Ok(self.drain_script_commands())
    }

    /// 按 [`HostPhase`] 调度已登记的脚本 System（同域串行）。
    ///
    /// 调度前按 `before`/`after` 拓扑排序，并检查组件访问声明冲突。
    /// 每个描述符调用其 `entry` 导出；调用后不自动提交命令（由宿主调用
    /// [`Self::apply_script_commands_to_world`]）。
    pub fn run_script_phase(&mut self, phase: HostPhase, host: &mut dyn HostHooks) -> Result<(), EngineError> {
        let jobs: Vec<(String, String, Option<ScriptSystemDescriptor>)> = self
            .script_systems
            .ordered_for_phase(phase)?
            .into_iter()
            .map(|s| (s.mod_id.to_string(), s.entry.to_string(), Some(s.clone())))
            .collect();
        for (mod_id, entry, desc) in jobs {
            let Some(m) = self.mods.get_mut(&mod_id)
            else {
                continue;
            };
            if !m.enabled {
                continue;
            }
            let Some(domain) = m.domain.as_mut()
            else {
                continue;
            };
            if !domain.enabled {
                continue;
            }
            let has_entry = domain.runtime.vm.module.functions.iter().any(|f| f.name == entry);
            if has_entry {
                self.shared.borrow_mut().begin_script_call(phase, desc.as_ref());
                let call_result = domain.call_in_phase(&entry, &[], phase, host);
                self.shared.borrow_mut().end_script_call();
                let _ = call_result?;
            }
        }
        Ok(())
    }

    /// 调度某一 phase 的脚本 System，派发事件，并把命令提交到 `world`。
    ///
    /// 调度前刷新查询快照，使本拍脚本读到当前世界；提交命令后再刷一次。
    pub fn run_script_systems(
        &mut self,
        phase: HostPhase,
        world: &mut spark_ecs::World,
        host: &mut dyn HostHooks,
    ) -> Result<CommandApplyReport, EngineError> {
        self.refresh_script_query(world);
        self.run_script_phase(phase, host)?;
        self.dispatch_script_events(host)?;
        Ok(self.apply_script_commands_to_world(world)?)
    }

    /// 编译/装载共用的宿主 schema：引擎内置 ABI + 插件宿主桩（`plugin` 命名空间）。
    fn build_host_schema(&self) -> HostSchema {
        let mut schema = crate::api::engine_host_schema();
        for name in self.plugins.native_names() {
            let qualified = format!("plugin.{name}");
            let known = schema.functions.iter().any(|f| f.id.qualified_name() == qualified);
            if !known {
                schema.insert(HostFunction::new(HostFunctionId::new("plugin", name, 1)));
            }
        }
        schema
    }
}

/// 装载模组脚本映像：显式 `.spkx` → 指纹磁盘缓存 → 源码编译（并回写缓存）。
fn load_mod_image(manifest: &ModManifest, root: &Path, host_schema: &HostSchema) -> Result<ExecutableImage, EngineError> {
    if let Some(artifact) = &manifest.artifact {
        let path = root.join(artifact);
        return load_spkx_checked(&path, host_schema);
    }
    let entry = manifest.entry.as_deref().ok_or_else(|| EngineError::Io { path: root.display().to_string(), kind: "missing_entry".into() })?;
    let entry_path = root.join(entry);
    let source = std::fs::read_to_string(&entry_path).map_err(|e| EngineError::from_io(entry_path.display().to_string(), e))?;
    let lang = resolve_language(manifest.language.as_deref(), entry)?;
    let request = CompilationRequest::for_mod(
        PackageId::new(manifest.id.as_str(), manifest.version.as_str()),
        lang,
        manifest.language.as_deref(),
        entry_path.clone(),
        entry,
        source.as_str(),
        host_schema.clone(),
    );
    let key = ArtifactCache::key_for(&request, &source);
    let cache_dir = root.join(".spark-cache");
    let cache_path = cache_dir.join(format!("{key:016x}.spkx"));
    if cache_path.is_file() {
        if let Ok(image) = load_spkx_checked(&cache_path, host_schema) {
            tracing::info!(
                event = "spark.engine.spkx_cache_hit",
                mod_id = %manifest.id,
                path = %cache_path.display()
            );
            return Ok(image);
        }
        tracing::warn!(
            event = "spark.engine.spkx_cache_miss",
            mod_id = %manifest.id,
            path = %cache_path.display()
        );
    }
    // 编译与装载必须共用同一份 schema（哈希校验）。
    let package = ScriptCompiler::new().compile(&request).map_err(EngineError::Script)?;
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        tracing::warn!(
            event = "spark.engine.spkx_cache_mkdir_failed",
            path = %cache_dir.display(),
            error = %e
        );
    }
    else if let Err(e) = package.image.write_spkx_file(&cache_path) {
        tracing::warn!(
            event = "spark.engine.spkx_cache_write_failed",
            path = %cache_path.display(),
            error = %e
        );
    }
    Ok(package.image)
}

fn load_spkx_checked(path: &Path, host_schema: &HostSchema) -> Result<ExecutableImage, EngineError> {
    let image =
        ExecutableImage::read_spkx_file(path).map_err(|e| EngineError::Io { path: path.display().to_string(), kind: e.code().into() })?;
    image.check_host_schema(host_schema).map_err(|e| EngineError::Script(ScriptError::compile_reason(e.code())))?;
    Ok(image)
}

fn find_dir_for_id(root: &Path, id: &str) -> Result<PathBuf, EngineError> {
    let candidate = root.join(id);
    if candidate.join("mod.von").is_file() {
        return Ok(candidate);
    }
    let rd = std::fs::read_dir(root).map_err(|e| EngineError::from_io(root.display().to_string(), e))?;
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

fn resolve_language(explicit: Option<&str>, entry: &str) -> Result<ScriptLanguage, EngineError> {
    if let Some(s) = explicit {
        return match s.to_ascii_lowercase().as_str() {
            "lua" => Ok(ScriptLanguage::Lua),
            "ruby" | "rgss" => Ok(ScriptLanguage::Ruby),
            "valkyrie" | "vk" | "v" => Ok(ScriptLanguage::Valkyrie),
            other => Err(EngineError::UnknownLanguage { token: other.into() }),
        };
    }
    let ext = Path::new(entry).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "lua" => Ok(ScriptLanguage::Lua),
        "rb" | "rgss" => Ok(ScriptLanguage::Ruby),
        "vk" | "valkyrie" | "vky" => Ok(ScriptLanguage::Valkyrie),
        "" => Err(EngineError::UnknownLanguage { token: "missing".into() }),
        other => Err(EngineError::UnknownLanguage { token: format!("ext:{other}") }),
    }
}
