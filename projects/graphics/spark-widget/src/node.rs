//! Widget 节点与种类。

use crate::{
    id::WidgetId,
    layout::{ComputedLayout, LayoutSpec},
    style::Style,
};

/// Widget 种类（retained）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    /// 树根；通常只作布局容器。
    Root,
    /// 通用容器（Flex / Grid 等）。
    Container,
    /// 静态文本标签。
    Label,
    /// 图片控件。
    Image,
    /// 可点击按钮。
    Button,
    /// 开关（开/关二态）。
    Toggle,
    /// 复选框。
    Checkbox,
    /// 单选按钮。
    Radio,
    /// 滑条。
    Slider,
    /// 进度条（通常只读）。
    ProgressBar,
    /// 单行文本输入。
    TextField,
    /// 多行文本输入。
    TextArea,
    /// 可滚动视口。
    ScrollView,
    /// 列表视图。
    ListView,
    /// 网格视图。
    GridView,
    /// 树形视图。
    TreeView,
    /// 选项卡容器。
    TabView,
    /// 可拖分栏。
    SplitView,
    /// 分隔线。
    Separator,
    /// 占位空白。
    Spacer,
    /// 面板容器（常带背景/边框）。
    Panel,
    /// 浮层提示。
    Tooltip,
    /// 弹出层。
    Popup,
    /// 模态遮罩层。
    Modal,
    /// 短暂提示条。
    Toast,
    /// 游戏侧自定义种类。
    Custom,
}

/// 节点携带的内容数据（文本等）。复杂绑定后续再拆。
#[derive(Debug, Clone)]
pub struct WidgetContent {
    /// 展示或编辑用文本（Label / Button / TextField 等）。
    pub text: Option<String>,
    /// 点击时经 router 派发的默认命令。
    pub click_command: Option<crate::command::UiCommand>,
    /// Checkbox / Toggle / Radio。
    pub checked: bool,
    /// Slider / ProgressBar 当前值。List 行可用作 index。
    pub value: f32,
    /// 数值下界（Slider / ProgressBar）。
    pub value_min: f32,
    /// 数值上界（Slider / ProgressBar）。
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
    /// 指针悬停。
    pub hovered: bool,
    /// 指针按下未释放。
    pub pressed: bool,
    /// 键盘 / 焦点环当前焦点。
    pub focused: bool,
    /// 禁用（不接收交互）。
    pub disabled: bool,
    /// 选中（列表行、Tab 等）。
    pub selected: bool,
    /// 勾选态（与 `WidgetContent::checked` 同步用）。
    pub checked: bool,
    /// 展开（树节点 / 折叠面板）。
    pub expanded: bool,
    /// 校验失败等无效态。
    pub invalid: bool,
    /// 是否参与布局与命中（隐藏则跳过）。
    pub visible: bool,
}

impl WidgetStateFlags {
    /// 可见且其余伪态为默认（未禁用、未聚焦等）。
    pub fn enabled_visible() -> Self {
        Self { visible: true, ..Self::default() }
    }
}

/// 树上的一个节点。
#[derive(Debug, Clone)]
pub struct WidgetNode {
    /// 本节点稳定 ID。
    pub id: WidgetId,
    /// 父节点；根为 `None`。
    pub parent: Option<WidgetId>,
    /// 直接子节点（挂载序即文档序）。
    pub children: Vec<WidgetId>,
    /// 控件种类。
    pub kind: WidgetKind,
    /// 可选稳定键，供 `child_by_key` / 列表复用查找。
    pub key: Option<String>,
    /// 声明式布局规格。
    pub layout: LayoutSpec,
    /// 上一帧布局引擎写出的几何结果。
    pub computed: ComputedLayout,
    /// 未解析的样式声明。
    pub style: Style,
    /// 交互伪态位。
    pub state: WidgetStateFlags,
    /// 文本、数值、拖放等内容载荷。
    pub content: WidgetContent,
    /// 是否可进入焦点环。
    pub focusable: bool,
    /// Tab 序：`>0` 按升序优先，`0` 跟文档序，`<0` 可点聚焦但不进 Tab 环。
    pub tab_index: i32,
    /// 方向键显式邻居（未设则按几何寻焦）。
    pub neighbors: crate::focus::Neighbors,
    /// 滚动偏移与内容尺寸（ScrollView 等）。
    pub scroll: crate::scroll::ScrollState,
    /// 所属 UI 层（HUD / Gui / Overlay 等）。
    pub layer: crate::runtime::UiLayer,
}

impl WidgetNode {
    /// 按种类构造节点；可聚焦控件会默认 `focusable = true`。
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
            tab_index: 0,
            neighbors: crate::focus::Neighbors::default(),
            scroll: crate::scroll::ScrollState::default(),
            layer: crate::runtime::UiLayer::Gui,
        }
    }

    /// 应用完整焦点策略（覆盖 `focusable` / `tab_index` / `neighbors`）。
    pub fn apply_focus_policy(&mut self, policy: crate::focus::FocusPolicy) {
        self.focusable = policy.focusable;
        self.tab_index = policy.tab_index;
        self.neighbors = policy.neighbors;
    }
}
