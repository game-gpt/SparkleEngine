//! Widget 节点与种类。

use crate::id::WidgetId;
use crate::layout::{ComputedLayout, LayoutSpec};
use crate::style::Style;

/// Widget 种类（retained）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    Root,
    Container,
    Label,
    Image,
    Button,
    Toggle,
    Checkbox,
    Radio,
    Slider,
    ProgressBar,
    TextField,
    TextArea,
    ScrollView,
    ListView,
    GridView,
    TreeView,
    TabView,
    SplitView,
    Separator,
    Spacer,
    Tooltip,
    Popup,
    Modal,
    Toast,
    Custom,
}

/// 交互与选择伪态位。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WidgetStateFlags {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub disabled: bool,
    pub selected: bool,
    pub checked: bool,
    pub expanded: bool,
    pub invalid: bool,
    pub visible: bool,
}

impl WidgetStateFlags {
    pub fn enabled_visible() -> Self {
        Self {
            visible: true,
            ..Self::default()
        }
    }
}

/// 树上的一个节点。
#[derive(Debug, Clone)]
pub struct WidgetNode {
    pub id: WidgetId,
    pub parent: Option<WidgetId>,
    pub children: Vec<WidgetId>,
    pub kind: WidgetKind,
    pub key: Option<String>,
    pub layout: LayoutSpec,
    pub computed: ComputedLayout,
    pub style: Style,
    pub state: WidgetStateFlags,
}

impl WidgetNode {
    pub fn new(id: WidgetId, kind: WidgetKind) -> Self {
        Self {
            id,
            parent: None,
            children: Vec::new(),
            kind,
            key: None,
            layout: LayoutSpec::default(),
            computed: ComputedLayout::default(),
            style: Style::default(),
            state: WidgetStateFlags::enabled_visible(),
        }
    }
}
