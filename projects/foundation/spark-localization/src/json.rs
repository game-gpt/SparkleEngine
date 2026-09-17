//! JSON 交换格式 ↔ [`LocalizationDocument`]。
//!
//! VON 与 JSON 必须落到同一中间模型；此处实现 JSON 侧，不引入私有语义。

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;
use thiserror::Error;

use crate::document::{
    ArgumentFormat, LocalizationDocument, MessageDefinition, MessageName, MessageNode, SelectKind,
};
use crate::locale::{LocaleId, LocaleParseError};

/// JSON 解析错误。
#[derive(Debug, Error)]
pub enum JsonError {
    #[error(transparent)]
    Locale(#[from] LocaleParseError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
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
                Some(JsonPattern::Nodes(nodes)) => {
                    Ok(MessageDefinition::Pattern(convert_nodes(&nodes)?))
                }
                None => Err(JsonError::Message(
                    "rich message needs `value` or `select`".into(),
                )),
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
    Ok(MessageDefinition::Select {
        argument: Arc::from(select.argument),
        kind,
        cases,
    })
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
                let MessageDefinition::Select {
                    argument,
                    kind,
                    cases,
                } = convert_select(JsonSelect {
                    argument: select.argument.clone(),
                    kind: select.kind.clone(),
                    cases: select.cases.clone(),
                })?
                else {
                    unreachable!("convert_select always returns Select");
                };
                return Ok(MessageNode::Select {
                    argument,
                    kind,
                    cases,
                });
            }
            if let Some(argument) = &obj.argument {
                return Ok(MessageNode::Argument {
                    name: Arc::from(argument.as_str()),
                    format: parse_format(obj.format.as_deref())?,
                });
            }
            if let Some(message) = &obj.message {
                return Ok(MessageNode::MessageRef {
                    name: MessageName::new(message.as_str()),
                    attribute: obj.attribute.as_ref().map(|s| Arc::from(s.as_str())),
                });
            }
            Err(JsonError::Message(
                "node object needs argument, message, or select".into(),
            ))
        }
    }
}

fn parse_select_kind(raw: Option<&str>) -> Result<SelectKind, JsonError> {
    match raw.unwrap_or("select") {
        "select" => Ok(SelectKind::Select),
        "cardinal" => Ok(SelectKind::Cardinal),
        "ordinal" => Ok(SelectKind::Ordinal),
        other => Err(JsonError::Message(format!("unknown select kind: {other}"))),
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
        other if other.starts_with("currency:") => Ok(ArgumentFormat::Currency {
            currency_code: Arc::from(other.trim_start_matches("currency:")),
        }),
        other => Err(JsonError::Message(format!("unknown argument format: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::compile_document;
    use crate::LocaleSnapshot;
    use crate::message::{MessageArgs, MessageRef, MessageValue};

    #[test]
    fn parses_simple_and_rich_json() {
        let json = r#"{
          "locale": "zh-Hans-CN",
          "namespace": "astracraft",
          "messages": {
            "menu.continue": "继续游戏",
            "player.welcome": { "value": "欢迎回来，{player_name}" },
            "inventory.item_count": {
              "select": {
                "argument": "count",
                "kind": "cardinal",
                "cases": {
                  "one": [{ "argument": "count", "format": "number" }, " item"],
                  "other": [{ "argument": "count", "format": "number" }, " items"]
                }
              }
            }
          }
        }"#;
        let doc = document_from_json_str(json).unwrap();
        assert_eq!(doc.locale.as_str(), "zh-Hans-CN");
        assert_eq!(doc.messages.len(), 3);
        let bundle = compile_document(&doc).unwrap();
        let snap = LocaleSnapshot::from_bundle(
            doc.locale.clone(),
            &doc.locale,
            &[doc.locale.clone()],
            1,
            bundle,
        );
        let mut args = MessageArgs::new();
        args.insert("count", MessageValue::Integer(1));
        let text = snap.format(&MessageRef::named("astracraft", "inventory.item_count"), &args);
        assert_eq!(text.text.as_ref(), "1 item");
    }
}
