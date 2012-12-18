//! Spark **Galgame / AVG** 特异化引擎壳。
//!
//! 对白队列、选项分支、旗标变量、背景/立绘图层槽、回看历史。
//! **默认**挂载 [`Live2dPlugin`]（占位后端可换），供模组脚本调用 `live2d_*` 原生函数。
//! **禁止**具体剧本、角色立绘资源表——那些属于游戏仓 / 模组数据。

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
    pub engine: SparkEngine,
    pub script: ScriptPlayer,
    pub flags: FlagStore,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialogue_and_choice() {
        let mut gal = GalgameEngine::new(".");
        gal.script.push_line(DialogueLine { speaker: Some("A".into()), text: "你好".into(), voice: None });
        gal.script.push_choices(vec![
            Choice { label: "去东".into(), set_flag: Some(("route".into(), 1.0)) },
            Choice { label: "去西".into(), set_flag: Some(("route".into(), 2.0)) },
        ]);
        assert!(matches!(gal.advance(), ScriptState::Line(_)));
        assert!(matches!(gal.advance(), ScriptState::Choices(_)));
        gal.script.choose(0, &mut gal.flags).unwrap();
        assert_eq!(gal.flags.get("route"), 1.0);
        assert!(matches!(gal.advance(), ScriptState::Ended));
    }

    #[test]
    fn live2d_plugin_registered_by_default() {
        let gal = GalgameEngine::new(".");
        assert!(gal.engine.plugins().contains("live2d"));
        assert!(gal.engine.plugins().native_names().iter().any(|n| *n == "live2d_load"));
        let id = gal.live2d().borrow_mut().backend.load("demo.model3.json").unwrap();
        gal.live2d().borrow_mut().backend.set_param(id, "ParamMouthOpenY", 0.8).unwrap();
        let v = gal.live2d().borrow().backend.get_param(id, "ParamMouthOpenY").unwrap();
        assert!((v - 0.8).abs() < 1e-5);
    }
}
