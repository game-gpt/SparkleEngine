//! 剪贴板抽象：Widget 不绑平台 API，由宿主注入。

/// 文本剪贴板：供 TextField 的 Copy / Cut / Paste 使用。
pub trait Clipboard: Send {
    /// 读取剪贴板文本；空或不可用时返回 `None`。
    fn get_text(&mut self) -> Option<String>;
    /// 写入剪贴板文本。
    fn set_text(&mut self, text: &str);
}

/// 进程内剪贴板（测试与无平台宿主时使用）。
#[derive(Debug, Default, Clone)]
pub struct MemoryClipboard {
    text: String,
}

impl MemoryClipboard {
    /// 创建空的进程内剪贴板。
    pub fn new() -> Self {
        Self::default()
    }
}

impl Clipboard for MemoryClipboard {
    fn get_text(&mut self) -> Option<String> {
        if self.text.is_empty() { None } else { Some(self.text.clone()) }
    }

    fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
}
