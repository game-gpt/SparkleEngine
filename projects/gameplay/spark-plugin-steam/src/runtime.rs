//! 共享运行时。

use std::{cell::RefCell, rc::Rc};

use crate::backend::SteamBackend;

/// 持有可替换 [`SteamBackend`] 的共享运行时，供插件闭包与宿主共用。
pub struct SteamRuntime {
    /// 当前后端实现（可在运行中替换，需注意借用）。
    pub backend: Box<dyn SteamBackend>,
}

impl SteamRuntime {
    /// 包装为 `Rc<RefCell<_>>`，便于多原生闭包共享可变访问。
    pub fn new(backend: Box<dyn SteamBackend>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { backend }))
    }
}
