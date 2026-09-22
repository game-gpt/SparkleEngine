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
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: UiColors::default(),
            menu_item: MenuItemColors::default(),
            typography: Typography::default(),
            spacing: Spacing::default(),
        }
    }
}

/// 透明底菜单文字按钮的 hover / pressed / idle 字色。
#[derive(Debug, Clone)]
pub struct MenuItemColors {
    /// 悬停或焦点时的字色（原版偏金黄）。
    pub hover: Color,
    /// 按下时的字色。
    pub pressed: Color,
}

impl Default for MenuItemColors {
    fn default() -> Self {
        Self {
            hover: Color::rgb(1.0, 0.92, 0.25),
            pressed: Color::rgb(0.85, 0.72, 0.12),
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
    /// 主文字色。
    pub foreground: Color,
    /// 强调色（按钮底、进度填充等）。
    pub accent: Color,
    /// 危险 / 错误色。
    pub danger: Color,
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
            foreground: Color::rgb(0.92, 0.94, 0.96),
            accent: Color::rgb(0.35, 0.65, 0.95),
            danger: Color::rgb(0.90, 0.30, 0.28),
            disabled: Color::rgb(0.45, 0.47, 0.50),
            border: Color::rgb(0.28, 0.32, 0.38),
            focus: Color::rgb(0.95, 0.85, 0.35),
            track: Color::rgb(0.22, 0.24, 0.28),
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
