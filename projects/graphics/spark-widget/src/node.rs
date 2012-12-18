//! Widget 节点与种类。

use crate::{
    id::WidgetId,
    layout::{ComputedLayout, LayoutSpec},
    style::Style,
};

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
    Panel,
    Tooltip,
    Popup,
    Modal,
    Toast,
    Custom,
}

/// 节点携带的内容数据（文本等）。复杂绑定后续再拆。
#[derive(Debug, Clone)]
pub struct WidgetContent {
    pub text: Option<String>,
    pub click_command: Option<crate::command::UiCommand>,
    /// Checkbox / Toggle / Radio。
    pub checked: bool,
    /// Slider / ProgressBar 当前值。List 行可用作 index。
    pub value: f32,
    pub value_min: f32,
    pub value_max: f32,
    /// 可作为拖放源。
    pub drag_source: bool,
    /// 可作为拖放目标。
    pub drop_target: bool,
    /// TextField 光标（字符索引）。
    pub cursor: usize,
    /// 选区锚点；`None` 表示无选区。
    pub sel_anchor: Option<usize>,
    /// IME 预编辑串（仅聚焦文本框时由 router 同步）。
    pub composition: String,
    /// Image 控件的资源句柄。
    pub image: Option<crate::asset::UiImage>,
    /// 点击在本节点停止继续冒泡（capture / target / bubble 任一阶段命中即停）。
    pub stop_click_propagation: bool,
    /// 点击时跳过 router 默认行为（勾选切换、滑条、`click_command` 等）。
    pub prevent_click_default: bool,
}

impl Default for WidgetContent {
    fn default() -> Self {
        Self {
            text: None,
            click_command: None,
            checked: false,
            value: 0.0,
            value_min: 0.0,
            value_max: 1.0,
            drag_source: false,
            drop_target: false,
            cursor: 0,
            sel_anchor: None,
            composition: String::new(),
            image: None,
            stop_click_propagation: false,
            prevent_click_default: false,
        }
    }
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
        Self { visible: true, ..Self::default() }
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
    pub content: WidgetContent,
    pub focusable: bool,
    pub neighbors: crate::focus::Neighbors,
    pub scroll: crate::scroll::ScrollState,
    pub layer: crate::runtime::UiLayer,
}

impl WidgetNode {
    pub fn new(id: WidgetId, kind: WidgetKind) -> Self {
        let focusable = matches!(
            kind,
            WidgetKind::Button
                | WidgetKind::Toggle
                | WidgetKind::Checkbox
                | WidgetKind::Radio
                | WidgetKind::Slider
                | WidgetKind::TextField
                | WidgetKind::TextArea
                | WidgetKind::ListView
                | WidgetKind::TabView
        );
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
            content: WidgetContent::default(),
            focusable,
            neighbors: crate::focus::Neighbors::default(),
            scroll: crate::scroll::ScrollState::default(),
            layer: crate::runtime::UiLayer::Gui,
        }
    }
}
