//! 剪贴板抽象：Widget 不绑平台 API，由宿主注入。

/// 文本剪贴板。
pub trait Clipboard: Send {
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, text: &str);
}

/// 进程内剪贴板（测试与无平台宿主时使用）。
#[derive(Debug, Default, Clone)]
pub struct MemoryClipboard {
    text: String,
}

impl MemoryClipboard {
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
