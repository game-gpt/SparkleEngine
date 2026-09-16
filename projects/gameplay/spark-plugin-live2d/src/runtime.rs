//! 运行时句柄。

use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::Live2dBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Live2dModelId(pub u32);

/// 共享运行时（插件 natives 闭包捕获）。
pub struct Live2dRuntime {
    pub backend: Box<dyn Live2dBackend>,
}

impl Live2dRuntime {
    pub fn new(backend: Box<dyn Live2dBackend>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { backend }))
    }
}
