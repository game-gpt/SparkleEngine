//! 可访问性语义。先建节点树，平台读屏可后接。

use spark_core::Rect;

use crate::id::WidgetId;

/// 控件角色。不绑定具体平台 API。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    None,
    Label,
    Button,
    CheckBox,
    Radio,
    Slider,
    TextField,
    ProgressBar,
    List,
    ListItem,
    Tab,
    TabPanel,
    Dialog,
    Tooltip,
    ScrollBar,
    Image,
    Heading,
    Separator,
}

/// 本帧登记的一个语义节点。
#[derive(Debug, Clone)]
pub struct AccessNode {
    pub id: WidgetId,
    pub role: Role,
    pub label: String,
    pub description: String,
    pub value: String,
    pub rect: Rect,
    pub disabled: bool,
    pub checked: Option<bool>,
    pub selected: bool,
    pub expanded: Option<bool>,
    pub focusable: bool,
    pub focused: bool,
}

impl AccessNode {
    pub fn new(id: WidgetId, role: Role) -> Self {
        Self {
            id,
            role,
            label: String::new(),
            description: String::new(),
            value: String::new(),
            rect: Rect::default(),
            disabled: false,
            checked: None,
            selected: false,
            expanded: None,
            focusable: false,
            focused: false,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = rect;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

/// 本帧可访问性树。每帧清空后由控件登记。
#[derive(Debug, Default, Clone)]
pub struct AccessTree {
    nodes: Vec<AccessNode>,
}

impl AccessTree {
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn push(&mut self, node: AccessNode) {
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[AccessNode] {
        &self.nodes
    }

    pub fn by_id(&self, id: WidgetId) -> Option<&AccessNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn focused(&self) -> Option<&AccessNode> {
        self.nodes.iter().find(|n| n.focused)
    }
}
