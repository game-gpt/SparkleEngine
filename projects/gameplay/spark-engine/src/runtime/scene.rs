//! 场景管理：内核拥有的场景生命周期，替代手写 `screen` / `Option<World>` 状态机。

use std::collections::HashMap;

use spark_ecs::World;

/// 游戏系统写入的场景切换请求队列（内核每仿真步消费）。
#[derive(Debug, Default)]
pub struct SceneRequests {
    /// 待处理命令（FIFO）。
    pub pending: Vec<SceneCommand>,
}

impl SceneRequests {
    /// 排队切换场景。
    pub fn transition(&mut self, to: impl Into<String>) {
        self.pending.push(SceneCommand::Transition { to: to.into() });
    }
}

/// 场景切换意图（由 Rust 系统或 Spark Script 经命令队列提交，内核在步末执行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneCommand {
    /// 切换到已注册场景 id。
    Transition {
        /// 目标场景名。
        to: String,
    },
}

/// 场景进入 / 离开回调表项。
struct SceneEntry {
    on_enter: Option<Box<dyn FnMut(&mut World) + Send>>,
    on_exit: Option<Box<dyn FnMut(&mut World) + Send>>,
}

/// 内核场景管理器。
pub struct SceneManager {
    entries: HashMap<String, SceneEntry>,
    current: Option<String>,
    pending: Option<String>,
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneManager {
    /// 空管理器。
    pub fn new() -> Self {
        Self { entries: HashMap::new(), current: None, pending: None }
    }

    /// 当前活动场景 id。
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// 注册场景进入回调（只操作共享 [`World`]）。
    pub fn on_enter(&mut self, id: impl Into<String>, f: impl FnMut(&mut World) + Send + 'static) {
        let id = id.into();
        let entry = self.entries.entry(id).or_insert(SceneEntry { on_enter: None, on_exit: None });
        entry.on_enter = Some(Box::new(f));
    }

    /// 注册场景离开回调。
    pub fn on_exit(&mut self, id: impl Into<String>, f: impl FnMut(&mut World) + Send + 'static) {
        let id = id.into();
        let entry = self.entries.entry(id).or_insert(SceneEntry { on_enter: None, on_exit: None });
        entry.on_exit = Some(Box::new(f));
    }

    /// 请求切换场景（下一步仿真前由内核消化）。
    pub fn request_transition(&mut self, to: impl Into<String>) {
        self.pending = Some(to.into());
    }

    /// 立即切换到已注册场景（装配期或测试用）。
    pub fn transition_now(&mut self, world: &mut World, to: impl Into<String>) {
        self.pending = Some(to.into());
        self.flush(world, None);
    }

    /// 消化挂起切换与单条场景命令。
    pub fn flush(&mut self, world: &mut World, command: Option<SceneCommand>) {
        if let Some(cmd) = command {
            match cmd {
                SceneCommand::Transition { to } => self.pending = Some(to),
            }
        }
        let Some(next) = self.pending.take() else {
            return;
        };
        if self.current.as_deref() == Some(next.as_str()) {
            return;
        }
        if let Some(cur) = self.current.clone() {
            if let Some(entry) = self.entries.get_mut(&cur) {
                if let Some(exit) = entry.on_exit.as_mut() {
                    exit(world);
                }
            }
        }
        self.current = Some(next.clone());
        if let Some(entry) = self.entries.get_mut(&next) {
            if let Some(enter) = entry.on_enter.as_mut() {
                enter(world);
            }
        }
    }
}
