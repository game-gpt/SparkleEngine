//! 运行时句柄。

use std::{cell::RefCell, rc::Rc};

use crate::backend::Live2dBackend;

/// 模型不透明句柄（后端分配的稠密 id）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Live2dModelId(pub u32);

/// 共享运行时（插件 natives 闭包捕获）。
pub struct Live2dRuntime {
    /// 当前后端实现（可替换为真实 Cubism）。
    pub backend: Box<dyn Live2dBackend>,
}

impl Live2dRuntime {
    /// 包装后端为 `Rc<RefCell<_>>`，供多闭包共享。
    pub fn new(backend: Box<dyn Live2dBackend>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { backend }))
    }
}
