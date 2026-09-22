//! 结构化日志事件（稳定 event 码 + 字段；文本仅由后端渲染）。

use std::sync::Arc;

use spark_diagnostics::{Error, ErrorArg, ErrorArgs};

/// 稳定日志事件标识（如 `spark.media.open_failed`）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(Arc<str>);

impl EventId {
    /// 从点分稳定码构造；调用方应保证码表全局唯一。
    pub fn new(dotted: impl Into<Arc<str>>) -> Self {
        Self(dotted.into())
    }

    /// 借出内部字符串切片。
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
    /// 本事件级别。
    pub level: crate::Level,
    /// 来源 target（模块路径或显式逻辑名）。
    pub target: &'static str,
    /// 稳定事件码。
    pub event: EventId,
    /// 键值字段表（机器类型，非自然语言）。
    pub fields: ErrorArgs,
    /// 可选关联的诊断 [`Error`]（其 `code` 会写入 `code_line`）。
    pub error: Option<Error>,
}

impl LogEvent {
    /// 构造无字段、无关联错误的事件。
    pub fn new(level: crate::Level, target: &'static str, event: impl Into<EventId>) -> Self {
        Self { level, target, event: event.into(), fields: ErrorArgs::new(), error: None }
    }

    /// 追加一个命名字段并返回 `self`（建造者模式）。
    pub fn field(mut self, name: impl Into<Arc<str>>, value: ErrorArg) -> Self {
        self.fields.insert(name, value);
        self
    }

    /// 整体替换字段表。
    pub fn with_fields(mut self, fields: ErrorArgs) -> Self {
        self.fields = fields;
        self
    }

    /// 挂上关联诊断错误。
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
