//! 共享运行时。

use std::cell::RefCell;
use std::rc::Rc;

use crate::backend::SteamBackend;

pub struct SteamRuntime {
    pub backend: Box<dyn SteamBackend>,
}

impl SteamRuntime {
    pub fn new(backend: Box<dyn SteamBackend>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { backend }))
    }
}
