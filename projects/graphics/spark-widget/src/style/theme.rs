//! 主题 tokens。

use spark_types::Color;

/// 全局主题：色板、字体阶与间距阶，供样式解析引用。
#[derive(Debug, Clone)]
pub struct Theme {
    /// 语义色板。
    pub colors: UiColors,
    /// 透明底菜单项（标题文字按钮等）伪态色。
    pub menu_item: MenuItemColors,
    /// 字号阶。
    pub typography: Typography,
    /// 间距阶。
    pub spacing: Spacing,
    /// 控件密度与几何度量。
    pub metrics: ControlMetrics,
    /// 普通按钮的默认视觉。
    pub button_treatment: ButtonTreatment,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: UiColors::default(),
            menu_item: MenuItemColors::default(),
            typography: Typography::default(),
            spacing: Spacing::default(),
            metrics: ControlMetrics::default(),
            button_treatment: ButtonTreatment::Accent,
        }
    }
}

impl Theme {
    /// 面向高密度桌面生产工具的石墨色暗色主题。
    pub fn editor_dark() -> Self {
        Self {
            colors: UiColors {
                background: Color::rgb(0.071, 0.078, 0.086),
                surface: Color::rgb(0.102, 0.110, 0.122),
                surface_raised: Color::rgb(0.133, 0.145, 0.161),
                control: Color::rgb(0.153, 0.169, 0.188),
                control_hover: Color::rgb(0.184, 0.204, 0.227),
                foreground: Color::rgb(0.847, 0.863, 0.882),
                foreground_secondary: Color::rgb(0.667, 0.690, 0.722),
                foreground_muted: Color::rgb(0.451, 0.478, 0.518),
                accent: Color::rgb(0.286, 0.475, 0.655),
                selection: Color::rgb(0.180, 0.298, 0.404),
                selection_inactive: Color::rgb(0.188, 0.208, 0.231),
                danger: Color::rgb(0.760, 0.310, 0.302),
                warning: Color::rgb(0.800, 0.584, 0.255),
                disabled: Color::rgb(0.176, 0.188, 0.204),
                border: Color::rgb(0.212, 0.227, 0.247),
                focus: Color::rgb(0.353, 0.584, 0.792),
                track: Color::rgb(0.118, 0.129, 0.145),
            },
            menu_item: MenuItemColors {
                hover: Color::rgb(0.925, 0.937, 0.949),
                pressed: Color::rgb(1.0, 1.0, 1.0),
                outline: Color::rgba(0.0, 0.0, 0.0, 0.0),
                outline_offsets: Vec::new(),
            },
            typography: Typography { body_size: 13.0, heading_size: 16.0, label_size: 12.0 },
            spacing: Spacing { xs: 2.0, sm: 4.0, md: 8.0, lg: 12.0 },
            metrics: ControlMetrics::compact(),
            button_treatment: ButtonTreatment::Quiet,
        }
    }
}

/// 普通按钮的默认视觉处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonTreatment {
    /// 使用强调色实底，兼容游戏与通用界面。
    Accent,
    /// 使用克制的控件表面，适合桌面生产工具。
    Quiet,
}

/// 透明底菜单文字按钮的 hover / pressed / idle 字色与描边。
#[derive(Debug, Clone)]
pub struct MenuItemColors {
    /// 悬停或焦点时的字色（原版偏金黄）。
    pub hover: Color,
    /// 按下时的字色。
    pub pressed: Color,
    /// 描边色（含 alpha）；透明菜单字在远景上保证可读。
    pub outline: Color,
    /// 描边像素偏移；空则不画描边。
    pub outline_offsets: Vec<(f32, f32)>,
}

impl Default for MenuItemColors {
    fn default() -> Self {
        Self {
            hover: Color::rgb(1.0, 0.92, 0.25),
            pressed: Color::rgb(0.85, 0.72, 0.12),
            outline: Color::rgba(0.0, 0.0, 0.0, 0.85),
            outline_offsets: vec![(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (1.0, 1.0)],
        }
    }
}

/// UI 语义颜色（背景、表面、强调、焦点环等）。
#[derive(Debug, Clone)]
pub struct UiColors {
    /// 窗口/页面底色。
    pub background: Color,
    /// 面板 / 卡片表面色。
    pub surface: Color,
    /// 浮层或抬升表面色。
    pub surface_raised: Color,
    /// 输入框与常规控件表面色。
    pub control: Color,
    /// 控件悬停表面色。
    pub control_hover: Color,
    /// 主文字色。
    pub foreground: Color,
    /// 次要但仍需清晰阅读的文字色。
    pub foreground_secondary: Color,
    /// 弱提示文字色。
    pub foreground_muted: Color,
    /// 强调色（按钮底、进度填充等）。
    pub accent: Color,
    /// 活动窗口中的选中色。
    pub selection: Color,
    /// 非活动窗口中的选中色。
    pub selection_inactive: Color,
    /// 危险 / 错误色。
    pub danger: Color,
    /// 警告色。
    pub warning: Color,
    /// 禁用态底色。
    pub disabled: Color,
    /// 边框默认色。
    pub border: Color,
    /// 焦点环色。
    pub focus: Color,
    /// 滑轨 / 轨道色。
    pub track: Color,
}

impl Default for UiColors {
    fn default() -> Self {
        Self {
            background: Color::rgb(0.08, 0.09, 0.11),
            surface: Color::rgb(0.14, 0.16, 0.20),
            surface_raised: Color::rgb(0.18, 0.20, 0.24),
            control: Color::rgb(0.18, 0.20, 0.24),
            control_hover: Color::rgb(0.22, 0.24, 0.29),
            foreground: Color::rgb(0.92, 0.94, 0.96),
            foreground_secondary: Color::rgb(0.74, 0.77, 0.81),
            foreground_muted: Color::rgb(0.55, 0.58, 0.62),
            accent: Color::rgb(0.35, 0.65, 0.95),
            selection: Color::rgb(0.25, 0.43, 0.62),
            selection_inactive: Color::rgb(0.28, 0.30, 0.34),
            danger: Color::rgb(0.90, 0.30, 0.28),
            warning: Color::rgb(0.92, 0.68, 0.25),
            disabled: Color::rgb(0.45, 0.47, 0.50),
            border: Color::rgb(0.28, 0.32, 0.38),
            focus: Color::rgb(0.95, 0.85, 0.35),
            track: Color::rgb(0.22, 0.24, 0.28),
        }
    }
}

/// 控件几何与密度度量。
#[derive(Debug, Clone)]
pub struct ControlMetrics {
    /// 紧凑行高。
    pub compact_height: f32,
    /// 常规控件高度。
    pub control_height: f32,
    /// 工具栏高度。
    pub toolbar_height: f32,
    /// 面板标题高度。
    pub panel_header_height: f32,
    /// 默认边框宽度。
    pub border_width: f32,
    /// 默认圆角半径。
    pub corner_radius: f32,
    /// 分栏线视觉宽度。
    pub splitter_width: f32,
    /// 分栏线指针命中宽度。
    pub splitter_hit_width: f32,
}

impl ControlMetrics {
    /// 高密度桌面工具度量。
    pub fn compact() -> Self {
        Self {
            compact_height: 22.0,
            control_height: 24.0,
            toolbar_height: 32.0,
            panel_header_height: 28.0,
            border_width: 1.0,
            corner_radius: 2.0,
            splitter_width: 1.0,
            splitter_hit_width: 6.0,
        }
    }
}

impl Default for ControlMetrics {
    fn default() -> Self {
        Self {
            compact_height: 24.0,
            control_height: 32.0,
            toolbar_height: 40.0,
            panel_header_height: 32.0,
            border_width: 1.0,
            corner_radius: 4.0,
            splitter_width: 1.0,
            splitter_hit_width: 6.0,
        }
    }
}

/// 主题字号阶。
#[derive(Debug, Clone)]
pub struct Typography {
    /// 正文字号。
    pub body_size: f32,
    /// 标题字号。
    pub heading_size: f32,
    /// 标签 / 辅助文字字号。
    pub label_size: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self { body_size: 16.0, heading_size: 22.0, label_size: 14.0 }
    }
}

/// 主题间距阶（布局 `gap` / padding 可引用）。
#[derive(Debug, Clone)]
pub struct Spacing {
    /// 极小间距。
    pub xs: f32,
    /// 小间距。
    pub sm: f32,
    /// 中等间距。
    pub md: f32,
    /// 大间距。
    pub lg: f32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self { xs: 4.0, sm: 8.0, md: 12.0, lg: 20.0 }
    }
}
