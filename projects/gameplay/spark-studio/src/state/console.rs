//! 控制台日志环形缓冲。

use std::collections::VecDeque;

/// 控制台消息行。
#[derive(Debug, Clone)]
pub struct ConsoleLog {
    lines: VecDeque<String>,
    cap: usize,
}

impl Default for ConsoleLog {
    fn default() -> Self {
        Self { lines: VecDeque::new(), cap: 128 }
    }
}

impl ConsoleLog {
    /// 追加一行；超出容量时丢弃最旧行。
    pub fn push(&mut self, line: impl Into<String>) {
        self.lines.push_back(line.into());
        while self.lines.len() > self.cap {
            self.lines.pop_front();
        }
    }

    /// 当前日志行（旧 → 新）。
    pub fn lines(&self) -> impl ExactSizeIterator<Item = &String> {
        self.lines.iter()
    }
}
