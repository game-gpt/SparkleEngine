//! JSON 交换格式 ↔ [`LocalizationDocument`]。
//!
//! VON 与 JSON 必须落到同一中间模型；此处实现 JSON 侧，不引入私有语义。

use std::{collections::BTreeMap, sync::Arc};

use serde::Deserialize;

use crate::{
    document::{ArgumentFormat, LocalizationDocument, MessageDefinition, MessageName, MessageNode, SelectKind},
    locale::{LocaleId, LocaleParseError},
};

/// JSON 解析错误（稳定码；无用户句子）。
#[derive(Debug)]
pub enum JsonError {
    /// `locale` 字段解析失败。
    Locale(LocaleParseError),
    /// serde_json 失败：只保留行列位置，不吞第三方 Display 句子。
    Serde {
        /// 1-based 行号。
        line: u64,
        /// 1-based 列号。
        column: u64,
    },
    /// 富消息对象既无 `value` 也无 `select`。
    RichMessageIncomplete,
    /// 节点对象缺少 `argument` / `message` / `select` 之一。
    NodeObjectIncomplete,
    /// 未知 select `kind` 字符串。
    UnknownSelectKind {
        /// 原始 kind 文本。
        kind: String,
    },
    /// 未知 argument `format` 字符串。
    UnknownArgumentFormat {
        /// 原始 format 文本。
        format: String,
    },
}

impl JsonError {
    /// 稳定机器码。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Locale(e) => e.code(),
            Self::Serde { .. } => "spark.localization.json_parse",
            Self::RichMessageIncomplete => "spark.localization.json_rich_incomplete",
            Self::NodeObjectIncomplete => "spark.localization.json_node_incomplete",
            Self::UnknownSelectKind { .. } => "spark.localization.json_unknown_select_kind",
            Self::UnknownArgumentFormat { .. } => "spark.localization.json_unknown_arg_format",
        }
    }

    /// 结构化参数（行列、kind、format 等）。
    pub fn args(&self) -> spark_types::ErrorArgs {
        use spark_types::{ErrorArg, ErrorArgs};
        match self {
            Self::Locale(e) => e.args(),
            Self::Serde { line, column } => {
                ErrorArgs::new().with("line", ErrorArg::Unsigned(*line)).with("column", ErrorArg::Unsigned(*column))
            }
            Self::RichMessageIncomplete | Self::NodeObjectIncomplete => ErrorArgs::new(),
            Self::UnknownSelectKind { kind } => ErrorArgs::new().with("kind", ErrorArg::String(Arc::from(kind.as_str()))),
            Self::UnknownArgumentFormat { format } => ErrorArgs::new().with("format", ErrorArg::String(Arc::from(format.as_str()))),
        }
    }
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for JsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Locale(e) => Some(e),
            _ => None,
        }
    }
}

impl From<LocaleParseError> for JsonError {
    fn from(value: LocaleParseError) -> Self {
        Self::Locale(value)
    }
}

impl From<serde_json::Error> for JsonError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde { line: value.line() as u64, column: value.column() as u64 }
    }
}

#[derive(Debug, Deserialize)]
struct JsonDocument {
    locale: String,
    namespace: String,
    messages: BTreeMap<String, JsonMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonMessage {
    Text(String),
    Rich(JsonRichMessage),
}

#[derive(Debug, Deserialize)]
struct JsonRichMessage {
    #[serde(default)]
    value: Option<JsonPattern>,
    #[serde(default)]
    select: Option<JsonSelect>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum JsonPattern {
    Text(String),
    Nodes(Vec<JsonNode>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum JsonNode {
    Text(String),
    Object(JsonNodeObject),
}

#[derive(Debug, Clone, Deserialize)]
struct JsonNodeObject {
    #[serde(default)]
    argument: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    attribute: Option<String>,
    #[serde(default)]
    select: Option<JsonSelect>,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonSelect {
    argument: String,
    #[serde(default)]
    kind: Option<String>,
    cases: BTreeMap<String, JsonPattern>,
}

/// 从 JSON 字节解析 [`LocalizationDocument`]。
pub fn document_from_json_slice(bytes: &[u8]) -> Result<LocalizationDocument, JsonError> {
    let raw: JsonDocument = serde_json::from_slice(bytes)?;
    document_from_json_value(raw)
}

/// 从 JSON 文本解析。
pub fn document_from_json_str(text: &str) -> Result<LocalizationDocument, JsonError> {
    document_from_json_slice(text.as_bytes())
}

fn document_from_json_value(raw: JsonDocument) -> Result<LocalizationDocument, JsonError> {
    let locale = LocaleId::parse(&raw.locale)?;
    let mut doc = LocalizationDocument::new(locale, raw.namespace);
    for (name, message) in raw.messages {
        doc.insert(name, convert_message(message)?);
    }
    Ok(doc)
}

fn convert_message(message: JsonMessage) -> Result<MessageDefinition, JsonError> {
    match message {
        JsonMessage::Text(text) => Ok(MessageDefinition::from_template_sugar(&text)),
        JsonMessage::Rich(rich) => {
            if let Some(select) = rich.select {
                return convert_select(select);
            }
            match rich.value {
                Some(JsonPattern::Text(text)) => Ok(MessageDefinition::from_template_sugar(&text)),
                Some(JsonPattern::Nodes(nodes)) => Ok(MessageDefinition::Pattern(convert_nodes(&nodes)?)),
                None => Err(JsonError::RichMessageIncomplete),
            }
        }
    }
}

fn convert_select(select: JsonSelect) -> Result<MessageDefinition, JsonError> {
    let kind = parse_select_kind(select.kind.as_deref())?;
    let mut cases = BTreeMap::new();
    for (key, pattern) in select.cases {
        cases.insert(Arc::from(key), convert_pattern(pattern)?);
    }
    Ok(MessageDefinition::Select { argument: Arc::from(select.argument), kind, cases })
}

fn convert_pattern(pattern: JsonPattern) -> Result<Vec<MessageNode>, JsonError> {
    match pattern {
        JsonPattern::Text(text) => match MessageDefinition::from_template_sugar(&text) {
            MessageDefinition::Text(t) => Ok(vec![MessageNode::Text(t)]),
            MessageDefinition::Pattern(nodes) => Ok(nodes),
            MessageDefinition::Select { .. } => Ok(vec![MessageNode::Text(Arc::from(text))]),
        },
        JsonPattern::Nodes(nodes) => convert_nodes(&nodes),
    }
}

fn convert_nodes(nodes: &[JsonNode]) -> Result<Vec<MessageNode>, JsonError> {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        out.push(convert_node(node)?);
    }
    Ok(out)
}

fn convert_node(node: &JsonNode) -> Result<MessageNode, JsonError> {
    match node {
        JsonNode::Text(text) => Ok(MessageNode::Text(Arc::from(text.as_str()))),
        JsonNode::Object(obj) => {
            if let Some(select) = &obj.select {
                let MessageDefinition::Select { argument, kind, cases } =
                    convert_select(JsonSelect { argument: select.argument.clone(), kind: select.kind.clone(), cases: select.cases.clone() })?
                else {
                    unreachable!("convert_select always returns Select");
                };
                return Ok(MessageNode::Select { argument, kind, cases });
            }
            if let Some(argument) = &obj.argument {
                return Ok(MessageNode::Argument { name: Arc::from(argument.as_str()), format: parse_format(obj.format.as_deref())? });
            }
            if let Some(message) = &obj.message {
                return Ok(MessageNode::MessageRef {
                    name: MessageName::new(message.as_str()),
                    attribute: obj.attribute.as_ref().map(|s| Arc::from(s.as_str())),
                });
            }
            Err(JsonError::NodeObjectIncomplete)
        }
    }
}

fn parse_select_kind(raw: Option<&str>) -> Result<SelectKind, JsonError> {
    match raw.unwrap_or("select") {
        "select" => Ok(SelectKind::Select),
        "cardinal" => Ok(SelectKind::Cardinal),
        "ordinal" => Ok(SelectKind::Ordinal),
        other => Err(JsonError::UnknownSelectKind { kind: other.to_string() }),
    }
}

fn parse_format(raw: Option<&str>) -> Result<ArgumentFormat, JsonError> {
    match raw.unwrap_or("none") {
        "none" | "" => Ok(ArgumentFormat::None),
        "number" => Ok(ArgumentFormat::Number),
        "percent" => Ok(ArgumentFormat::Percent),
        "date" => Ok(ArgumentFormat::Date),
        "time" => Ok(ArgumentFormat::Time),
        "datetime" => Ok(ArgumentFormat::DateTime),
        "relative-time" => Ok(ArgumentFormat::RelativeTime),
        "list" => Ok(ArgumentFormat::List),
        other if other.starts_with("currency:") => {
            Ok(ArgumentFormat::Currency { currency_code: Arc::from(other.trim_start_matches("currency:")) })
        }
        other => Err(JsonError::UnknownArgumentFormat { format: other.to_string() }),
    }
}
