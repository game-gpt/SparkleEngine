//! Spark Script 执行域：热更 Mod、内容与脚本系统调度。

use std::path::{Path, PathBuf};

use spark_ecs::World;
use spark_script::HostPhase;
use spark_vm::{HostHooks, StdHost};

use crate::{CommandApplyReport, EngineError, ExecutionProfile, SparkEngine};

/// Spark Script 包（Mod / 热更脚本）的运行时域。
///
/// 与 Rust 域共享同一个 [`World`]；能力在绑定期解析，结构变更经命令缓冲在 phase 边界提交。
pub struct SparkScriptDomain {
    engine: Option<SparkEngine>,
    mods_root: Option<PathBuf>,
    execution_profile: ExecutionProfile,
}

impl Default for SparkScriptDomain {
    fn default() -> Self {
        Self::new()
    }
}

impl SparkScriptDomain {
    /// 空域（无已装载脚本包）。
    pub fn new() -> Self {
        Self { engine: None, mods_root: None, execution_profile: ExecutionProfile::default() }
    }

    /// 设置脚本包执行策略并同步到已挂载引擎壳。
    pub fn set_execution_profile(&mut self, profile: ExecutionProfile) {
        self.execution_profile = profile;
        if let Some(engine) = &mut self.engine {
            engine.set_execution_profile(profile);
        }
    }

    /// 是否已装载至少一个脚本包根。
    pub fn is_loaded(&self) -> bool {
        self.engine.is_some()
    }

    /// 只读访问底层引擎壳（模组、钩子、脚本 System 表）。
    pub fn engine(&self) -> Option<&SparkEngine> {
        self.engine.as_ref()
    }

    /// 可变访问底层引擎壳。
    pub fn engine_mut(&mut self) -> Option<&mut SparkEngine> {
        self.engine.as_mut()
    }

    /// 确保已挂载空引擎壳（仅 System 登记表；无模组字节码）。
    ///
    /// 供原生插件在 `load_script_package` 之前登记 [`ScriptSystemDescriptor`]。
    pub fn ensure_engine(&mut self) -> &mut SparkEngine {
        if self.engine.is_none() {
            let root = self.mods_root.clone().unwrap_or_else(|| PathBuf::from("."));
            let mut engine = SparkEngine::new(root);
            engine.set_execution_profile(self.execution_profile);
            self.engine = Some(engine);
        }
        self.engine.as_mut().expect("just inserted")
    }

    /// 装载脚本包根目录并扫描全部 `mod.von`。
    pub fn load_package_root(&mut self, mods_root: impl AsRef<Path>) -> Result<(), EngineError> {
        let root = mods_root.as_ref().to_path_buf();
        let mut engine = SparkEngine::new(&root);
        engine.load_all()?;
        self.engine = Some(engine);
        self.mods_root = Some(root);
        Ok(())
    }

    /// 运行某一 Spark Script 相位并把命令提交到共享世界。
    pub fn run_phase(&mut self, phase: HostPhase, world: &mut World, host: &mut dyn HostHooks) -> Result<(), EngineError> {
        if let Some(engine) = &mut self.engine {
            engine.run_script_systems(phase, world, host)?;
        }
        Ok(())
    }

    /// 执行单个已登记脚本 System（不提交命令；由相位边界统一 apply）。
    pub fn run_script_descriptor(
        &mut self,
        desc: &crate::ScriptSystemDescriptor,
        _world: &mut World,
        host: &mut dyn HostHooks,
    ) -> Result<(), EngineError> {
        if let Some(engine) = &mut self.engine {
            engine.run_script_descriptor(desc, host)?;
        }
        Ok(())
    }

    /// 派发各模组事件 inbox（在 LateUpdate 之后由调度器调用）。
    pub fn dispatch_events(&mut self, host: &mut dyn HostHooks) -> Result<(), EngineError> {
        if let Some(engine) = &mut self.engine {
            engine.dispatch_script_events(host)?;
        }
        Ok(())
    }

    /// 帧同步点：取出并应用脚本命令缓冲。
    pub fn apply_commands(&mut self, world: &mut World) -> Result<CommandApplyReport, EngineError> {
        if let Some(engine) = &mut self.engine {
            return engine.apply_script_commands_to_world(world);
        }
        Ok(CommandApplyReport::default())
    }

    /// 帧边界：本地化提交与事件双缓冲（无脚本包时为空操作）。
    pub fn begin_frame(&mut self) -> Option<spark_localization::LocaleChanged> {
        self.engine.as_mut().and_then(|e| e.begin_frame())
    }

    /// 使用默认 [`StdHost`] 运行相位（测试 / 无自定义宿主钩子）。
    pub fn run_phase_std(&mut self, phase: HostPhase, world: &mut World) -> Result<(), EngineError> {
        let mut host = StdHost;
        self.run_phase(phase, world, &mut host)
    }
}
