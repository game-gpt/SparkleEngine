//! 背景 / 立绘等图层槽（资源 ID 不透明）。

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayerId(pub String);

#[derive(Debug, Clone)]
pub struct LayerSlot {
    pub asset: Option<String>,
    pub visible: bool,
}

#[derive(Debug, Default)]
pub struct LayerStack {
    layers: HashMap<LayerId, LayerSlot>,
}

impl LayerStack {
    pub fn set(&mut self, id: impl Into<String>, asset: Option<String>) {
        let id = LayerId(id.into());
        self.layers.insert(
            id,
            LayerSlot {
                asset,
                visible: true,
            },
        );
    }

    pub fn hide(&mut self, id: &str) {
        if let Some(s) = self.layers.get_mut(&LayerId(id.into())) {
            s.visible = false;
        }
    }

    pub fn get(&self, id: &str) -> Option<&LayerSlot> {
        self.layers.get(&LayerId(id.into()))
    }

    pub fn clear(&mut self) {
        self.layers.clear();
    }
}
