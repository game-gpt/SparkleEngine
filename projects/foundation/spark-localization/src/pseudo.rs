//! 伪本地化：扩张、加长与 RTL 镜像，用于暴露硬编码布局假设。

use std::sync::Arc;

use crate::{
    document::{LocalizationDocument, MessageDefinition, MessageNode},
    locale::LocaleId,
};

/// 伪 Locale 变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoKind {
    /// 重音扩张（保留可读性，拉长字形宽度）。
    Accents,
    /// 超长文本（约 1.5×–2×）。
    Long,
    /// RTL 镜像包装（加入 RLM / 方向隔离标记）。
    RtlMirror,
}

impl PseudoKind {
    pub fn locale_tag(self) -> &'static str {
        match self {
            Self::Accents => "en-XA",
            Self::Long => "en-XB",
            Self::RtlMirror => "en-XC",
        }
    }
}

/// 由基线英文（或任意）文档生成伪本地化文档。
pub fn generate_pseudo(source: &LocalizationDocument, kind: PseudoKind) -> LocalizationDocument {
    let locale = LocaleId::parse(kind.locale_tag()).expect("pseudo locale tags are valid");
    let mut out = LocalizationDocument::new(locale, source.namespace.as_str());
    for (name, def) in &source.messages {
        out.insert(name.as_str(), transform_definition(def, kind));
    }
    out
}

fn transform_definition(def: &MessageDefinition, kind: PseudoKind) -> MessageDefinition {
    match def {
        MessageDefinition::Text(text) => MessageDefinition::Text(Arc::from(transform_text(text, kind))),
        MessageDefinition::Pattern(nodes) => MessageDefinition::Pattern(transform_nodes(nodes, kind)),
        MessageDefinition::Select { argument, kind: select_kind, cases } => {
            let mut new_cases = cases.clone();
            for nodes in new_cases.values_mut() {
                *nodes = transform_nodes(nodes, kind);
            }
            MessageDefinition::Select { argument: argument.clone(), kind: *select_kind, cases: new_cases }
        }
    }
}

fn transform_nodes(nodes: &[MessageNode], kind: PseudoKind) -> Vec<MessageNode> {
    nodes
        .iter()
        .map(|node| match node {
            MessageNode::Text(text) => MessageNode::Text(Arc::from(transform_text(text, kind))),
            MessageNode::Argument { name, format } => MessageNode::Argument { name: name.clone(), format: format.clone() },
            MessageNode::MessageRef { name, attribute } => MessageNode::MessageRef { name: name.clone(), attribute: attribute.clone() },
            MessageNode::Select { argument, kind: select_kind, cases } => {
                let mut new_cases = cases.clone();
                for child in new_cases.values_mut() {
                    *child = transform_nodes(child, kind);
                }
                MessageNode::Select { argument: argument.clone(), kind: *select_kind, cases: new_cases }
            }
        })
        .collect()
}

fn transform_text(input: &str, kind: PseudoKind) -> String {
    match kind {
        PseudoKind::Accents => accentuate(input),
        PseudoKind::Long => {
            let accented = accentuate(input);
            format!("[{accented} >>>]")
        }
        PseudoKind::RtlMirror => {
            // U+202E RLO … U+202C PDF：强制视觉镜像，便于发现未声明方向的布局。
            format!("\u{202E}{input}\u{202C}")
        }
    }
}

fn accentuate(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for ch in input.chars() {
        out.push(match ch {
            'a' => 'á',
            'e' => 'é',
            'i' => 'í',
            'o' => 'ó',
            'u' => 'ú',
            'A' => 'Á',
            'E' => 'É',
            'I' => 'Í',
            'O' => 'Ó',
            'U' => 'Ú',
            'c' => 'ç',
            'C' => 'Ç',
            'n' => 'ñ',
            'N' => 'Ñ',
            other => other,
        });
    }
    out
}
