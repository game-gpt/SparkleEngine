//! 结构化日志事件（稳定 event 码 + 字段；文本仅由后端渲染）。

use std::sync::Arc;

use spark_diagnostics::{Error, ErrorArg, ErrorArgs};

/// 稳定日志事件标识（如 `spark.media.open_failed`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(Arc<str>);

impl EventId {
    pub fn new(dotted: impl Into<Arc<str>>) -> Self {
        Self(dotted.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for EventId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// 结构化日志事件。
///
/// `event` 永远是稳定码，可用于聚合；可选本地化消息只是渲染结果，不是权威字段。
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub level: crate::Level,
    pub target: &'static str,
    pub event: EventId,
    pub fields: ErrorArgs,
    pub error: Option<Error>,
}

impl LogEvent {
    pub fn new(level: crate::Level, target: &'static str, event: impl Into<EventId>) -> Self {
        Self { level, target, event: event.into(), fields: ErrorArgs::new(), error: None }
    }

    pub fn field(mut self, name: impl Into<Arc<str>>, value: ErrorArg) -> Self {
        self.fields.insert(name, value);
        self
    }

    pub fn with_fields(mut self, fields: ErrorArgs) -> Self {
        self.fields = fields;
        self
    }

    pub fn with_error(mut self, error: Error) -> Self {
        self.error = Some(error);
        self
    }

    /// 机器可读摘要行（非用户 Locale 句子）。
    pub fn code_line(&self) -> String {
        let mut line = format!("[{}] [{}] event={}", self.level.as_str(), self.target, self.event);
        for (k, v) in self.fields.iter() {
            line.push(' ');
            line.push_str(k);
            line.push('=');
            match v {
                ErrorArg::Integer(n) => line.push_str(&n.to_string()),
                ErrorArg::Unsigned(n) => line.push_str(&n.to_string()),
                ErrorArg::Float(n) => line.push_str(&n.to_string()),
                ErrorArg::String(s) | ErrorArg::AssetKey(s) | ErrorArg::Path(s) | ErrorArg::TypeName(s) => {
                    line.push_str(s);
                }
                ErrorArg::Opcode(op) => line.push_str(&format!("0x{op:02x}")),
                ErrorArg::EntityBits(bits) => line.push_str(&bits.to_string()),
                ErrorArg::Bool(b) => line.push_str(if *b { "true" } else { "false" }),
                ErrorArg::Span(span) => {
                    line.push_str(&format!("{}..{}", span.start, span.end));
                }
            }
        }
        if let Some(err) = &self.error {
            line.push_str(" error=");
            line.push_str(&err.code.to_string());
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;

    #[test]
    fn code_line_keeps_stable_event() {
        let ev = LogEvent::new(Level::Error, "media", "spark.media.open_failed").field("path", ErrorArg::Path(Arc::from("clips/intro.bin")));
        let line = ev.code_line();
        assert!(line.contains("event=spark.media.open_failed"));
        assert!(line.contains("path=clips/intro.bin"));
        assert!(!line.contains("无法"));
    }
}
