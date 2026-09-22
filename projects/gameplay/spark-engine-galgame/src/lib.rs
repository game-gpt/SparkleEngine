//! Spark **Galgame / AVG** 特异化引擎壳。
//!
//! 对白队列、选项分支、旗标变量、背景/立绘图层槽、回看历史。
//! **默认**挂载 [`Live2dPlugin`]（占位后端可换），供模组脚本调用 `live2d_*` 原生函数。
//! **禁止**具体剧本、角色立绘资源表——那些属于游戏仓 / 模组数据。

#![forbid(missing_docs)]
mod flags;
mod layers;
mod script;

pub use flags::FlagStore;
pub use layers::{LayerId, LayerStack};
pub use script::{Choice, DialogueLine, ScriptPlayer, ScriptState};
pub use spark_plugin_live2d::{LIVE2D_NATIVES, Live2dBackend, Live2dModelId, Live2dPlugin, Live2dRuntime, NullLive2dBackend};

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use spark_engine::SparkEngine;

/// Galgame 会话（默认已注册 Live2D 脚本插件）。
pub struct GalgameEngine {
    /// 底层模组 / 帧循环宿主。
    pub engine: SparkEngine,
    /// 对白与选项播放器（含回看历史）。
    pub script: ScriptPlayer,
    /// 剧本旗标 / 数值变量表。
    pub flags: FlagStore,
    /// 背景 / 立绘等图层槽。
    pub layers: LayerStack,
    live2d: Rc<RefCell<Live2dRuntime>>,
}

impl GalgameEngine {
    /// 使用 [`NullLive2dBackend`] 占位；宿主可再经 [`Self::live2d`] 换真实 Cubism 后端。
    pub fn new(mods_root: impl Into<PathBuf>) -> Self {
        Self::with_live2d_backend(mods_root, Box::new(NullLive2dBackend::default()))
    }

    /// 指定 Live2D 后端后构造（仍自动 `register_plugin`）。
    pub fn with_live2d_backend(mods_root: impl Into<PathBuf>, backend: Box<dyn Live2dBackend>) -> Self {
        let live2d = Live2dRuntime::new(backend);
        let mut engine = SparkEngine::new(mods_root);
        engine.register_plugin(Box::new(Live2dPlugin::new(Rc::clone(&live2d)))).expect("默认 Live2D 插件 id 不应冲突");
        Self { engine, script: ScriptPlayer::default(), flags: FlagStore::default(), layers: LayerStack::default(), live2d }
    }

    /// 共享 Live2D 运行时（与脚本 `live2d_*` 同一后端）。
    pub fn live2d(&self) -> &Rc<RefCell<Live2dRuntime>> {
        &self.live2d
    }

    /// 推进：无选项时吃掉一行对白；有选项时需先 [`ScriptPlayer::choose`]。
    pub fn advance(&mut self) -> ScriptState {
        self.script.advance(&mut self.flags)
    }
}
