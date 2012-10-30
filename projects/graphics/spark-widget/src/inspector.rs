//! 把 `UiState` 接到 `spark-debugger` 检查器。

use spark_debugger::{InspectField, InspectNode, Inspector};

use crate::state::UiState;

/// 只读检查器：热/活动/焦点、冲突 ID、可访问性节点。
pub struct UiInspector<'a> {
    pub state: &'a UiState,
}

impl<'a> UiInspector<'a> {
    pub fn new(state: &'a UiState) -> Self {
        Self { state }
    }
}

impl Inspector for UiInspector<'_> {
    fn collect(&self) -> Vec<InspectNode> {
        let mut nodes = Vec::new();
        nodes.push(InspectNode {
            label: "ui.focus".into(),
            fields: vec![
                field("hot", opt_id(self.state.hot)),
                field("active", opt_id(self.state.active)),
                field("focused", opt_id(self.state.focused)),
                field("captured", opt_id(self.state.captured)),
                field("focus_source", format!("{:?}", self.state.focus_source)),
            ],
        });
        let conflicts = self.state.id_conflicts();
        if !conflicts.is_empty() {
            nodes.push(InspectNode {
                label: "ui.id_conflicts".into(),
                fields: conflicts
                    .iter()
                    .enumerate()
                    .map(|(i, id)| InspectField {
                        name: "id",
                        value: format!("{i}:{:x}", id.raw()),
                    })
                    .collect(),
            });
        }
        nodes.push(InspectNode {
            label: "ui.prefs".into(),
            fields: vec![
                field("ui_scale", format!("{:.2}", self.state.prefs.ui_scale)),
                field("font_scale", format!("{:.2}", self.state.prefs.font_scale)),
                field("reduced_motion", self.state.prefs.reduced_motion.to_string()),
                field("high_contrast", self.state.prefs.high_contrast.to_string()),
            ],
        });
        for node in self.state.access.nodes() {
            nodes.push(InspectNode {
                label: format!("access.{:?}", node.role),
                fields: vec![
                    field("id", format!("{:x}", node.id.raw())),
                    field("label", node.label.clone()),
                    field("value", node.value.clone()),
                    field("focused", node.focused.to_string()),
                    field("disabled", node.disabled.to_string()),
                ],
            });
        }
        nodes
    }
}

fn field(name: &'static str, value: impl Into<String>) -> InspectField {
    InspectField {
        name,
        value: value.into(),
    }
}

fn opt_id(id: Option<crate::id::WidgetId>) -> String {
    id.map(|id| format!("{:x}", id.raw()))
        .unwrap_or_else(|| "-".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{AccessNode, Role};
    use crate::id::WidgetId;

    #[test]
    fn inspector_lists_access_nodes() {
        let mut state = UiState::new();
        state.access.push(
            AccessNode::new(WidgetId::from_hashable("a"), Role::Button)
                .label("开始")
                .focused(true),
        );
        let nodes = UiInspector::new(&state).collect();
        assert!(nodes.iter().any(|n| n.label.starts_with("access.")));
        assert!(nodes.iter().any(|n| n.label == "ui.focus"));
    }
}
