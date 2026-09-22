//! 背景 / 立绘等图层槽（资源 ID 不透明）。
//!
//! 本模块只维护「槽位 → 资源键 + 可见性」，不加载纹理、不参与绘制列表。

use std::collections::HashMap;

/// 图层槽标识（游戏侧自定义字符串）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayerId(pub String);

/// 单个图层槽的当前内容。
#[derive(Debug, Clone)]
pub struct LayerSlot {
    /// 不透明资源键；`None` 表示槽为空但仍可存在于表中。
    pub asset: Option<String>,
    /// 是否参与显示；[`LayerStack::hide`] 只改此标志。
    pub visible: bool,
}

/// 图层槽集合。
#[derive(Debug, Default)]
pub struct LayerStack {
    layers: HashMap<LayerId, LayerSlot>,
}

impl LayerStack {
    /// 设置槽位资源并置为可见；同名键覆盖。
    pub fn set(&mut self, id: impl Into<String>, asset: Option<String>) {
        let id = LayerId(id.into());
        self.layers.insert(id, LayerSlot { asset, visible: true });
    }

    /// 将已有槽位置为不可见；槽不存在时为 no-op。
    pub fn hide(&mut self, id: &str) {
        if let Some(s) = self.layers.get_mut(&LayerId(id.into())) {
            s.visible = false;
        }
    }

    /// 按槽名查询；不存在返回 `None`。
    pub fn get(&self, id: &str) -> Option<&LayerSlot> {
        self.layers.get(&LayerId(id.into()))
    }

    /// 清空全部槽位。
    pub fn clear(&mut self) {
        self.layers.clear();
    }
}
