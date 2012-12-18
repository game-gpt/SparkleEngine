//! 构建期文档检查：重复键、参数一致性、选择分支与循环引用。

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::Arc,
};

use crate::{
    document::{LocalizationDocument, MessageDefinition, MessageName, MessageNode, SelectKind},
    locale::LocaleId,
    message::NamespaceId,
};

/// 检查发现问题。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckIssueKind {
    DuplicateKey,
    MissingKey,
    ExtraKey,
    ArgumentMismatch,
    NamespaceMismatch,
    MissingSelectCase,
    Cycle,
    EmptyMessage,
}

/// 单条检查结果。
///
/// `token` 仅承载机器可读附加事实（标识符链、参数名列表等），禁止用户句子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckIssue {
    pub kind: CheckIssueKind,
    pub namespace: NamespaceId,
    pub locale: Option<LocaleId>,
    pub message: Option<MessageName>,
    pub token: Option<Arc<str>>,
}

/// 一组同命名空间、多 Locale 文档的检查报告。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckReport {
    pub issues: Vec<CheckIssue>,
}

impl CheckReport {
    pub fn is_ok(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.issues.len()
    }
}

/// 检查单个文档内部一致性。
pub fn check_document(doc: &LocalizationDocument) -> CheckReport {
    let mut report = CheckReport::default();
    for (name, def) in &doc.messages {
        if is_empty_definition(def) {
            report.issues.push(CheckIssue {
                kind: CheckIssueKind::EmptyMessage,
                namespace: doc.namespace.clone(),
                locale: Some(doc.locale.clone()),
                message: Some(name.clone()),
                token: None,
            });
        }
        let mut stack = Vec::new();
        let mut visiting = HashSet::new();
        detect_cycles(doc, name, def, &mut stack, &mut visiting, &mut report);
        check_select_cases(doc, name, def, &mut report);
    }
    report
}

/// 以 `baseline` 为权威键集，对比其它 Locale 文档。
pub fn check_locale_set(baseline: &LocalizationDocument, others: &[LocalizationDocument]) -> CheckReport {
    let mut report = check_document(baseline);
    let base_keys: BTreeSet<_> = baseline.messages.keys().cloned().collect();
    let base_args = collect_arg_schemas(baseline);

    for doc in others {
        if doc.namespace != baseline.namespace {
            report.issues.push(CheckIssue {
                kind: CheckIssueKind::NamespaceMismatch,
                namespace: doc.namespace.clone(),
                locale: Some(doc.locale.clone()),
                message: None,
                token: Some(Arc::from(format!("{}!={}", doc.namespace.as_str(), baseline.namespace.as_str()))),
            });
            continue;
        }
        report.issues.extend(check_document(doc).issues);

        let keys: BTreeSet<_> = doc.messages.keys().cloned().collect();
        for missing in base_keys.difference(&keys) {
            report.issues.push(CheckIssue {
                kind: CheckIssueKind::MissingKey,
                namespace: doc.namespace.clone(),
                locale: Some(doc.locale.clone()),
                message: Some(missing.clone()),
                token: None,
            });
        }
        for extra in keys.difference(&base_keys) {
            report.issues.push(CheckIssue {
                kind: CheckIssueKind::ExtraKey,
                namespace: doc.namespace.clone(),
                locale: Some(doc.locale.clone()),
                message: Some(extra.clone()),
                token: None,
            });
        }

        let other_args = collect_arg_schemas(doc);
        for (message, expected) in &base_args {
            if let Some(actual) = other_args.get(message) {
                if actual != expected {
                    report.issues.push(CheckIssue {
                        kind: CheckIssueKind::ArgumentMismatch,
                        namespace: doc.namespace.clone(),
                        locale: Some(doc.locale.clone()),
                        message: Some(message.clone()),
                        token: Some(Arc::from(format!("{}!={}", join_names(expected), join_names(actual)))),
                    });
                }
            }
        }
    }

    report
}

fn is_empty_definition(def: &MessageDefinition) -> bool {
    match def {
        MessageDefinition::Text(text) => text.is_empty(),
        MessageDefinition::Pattern(nodes) => nodes.is_empty(),
        MessageDefinition::Select { cases, .. } => cases.is_empty(),
    }
}

fn check_select_cases(doc: &LocalizationDocument, name: &MessageName, def: &MessageDefinition, report: &mut CheckReport) {
    match def {
        MessageDefinition::Select { kind, cases, .. } => {
            require_other_case(doc, name, *kind, cases, report);
            for nodes in cases.values() {
                for node in nodes {
                    walk_select_in_nodes(doc, name, node, report);
                }
            }
        }
        MessageDefinition::Pattern(nodes) => {
            for node in nodes {
                walk_select_in_nodes(doc, name, node, report);
            }
        }
        MessageDefinition::Text(_) => {}
    }
}

fn walk_select_in_nodes(doc: &LocalizationDocument, name: &MessageName, node: &MessageNode, report: &mut CheckReport) {
    if let MessageNode::Select { kind, cases, .. } = node {
        require_other_case(doc, name, *kind, cases, report);
        for nodes in cases.values() {
            for child in nodes {
                walk_select_in_nodes(doc, name, child, report);
            }
        }
    }
}

fn require_other_case(
    doc: &LocalizationDocument,
    name: &MessageName,
    kind: SelectKind,
    cases: &BTreeMap<Arc<str>, Vec<MessageNode>>,
    report: &mut CheckReport,
) {
    if matches!(kind, SelectKind::Cardinal | SelectKind::Ordinal | SelectKind::Select) && !cases.contains_key("other") {
        report.issues.push(CheckIssue {
            kind: CheckIssueKind::MissingSelectCase,
            namespace: doc.namespace.clone(),
            locale: Some(doc.locale.clone()),
            message: Some(name.clone()),
            token: Some(Arc::from("missing_other")),
        });
    }
}

fn detect_cycles(
    doc: &LocalizationDocument,
    name: &MessageName,
    def: &MessageDefinition,
    stack: &mut Vec<MessageName>,
    visiting: &mut HashSet<MessageName>,
    report: &mut CheckReport,
) {
    if !visiting.insert(name.clone()) {
        report.issues.push(CheckIssue {
            kind: CheckIssueKind::Cycle,
            namespace: doc.namespace.clone(),
            locale: Some(doc.locale.clone()),
            message: Some(name.clone()),
            token: Some(Arc::from(join_stack(stack))),
        });
        return;
    }
    stack.push(name.clone());
    match def {
        MessageDefinition::Text(_) => {}
        MessageDefinition::Pattern(nodes) => walk_refs(doc, nodes, stack, visiting, report),
        MessageDefinition::Select { cases, .. } => {
            for nodes in cases.values() {
                walk_refs(doc, nodes, stack, visiting, report);
            }
        }
    }
    stack.pop();
    visiting.remove(name);
}

fn walk_refs(
    doc: &LocalizationDocument,
    nodes: &[MessageNode],
    stack: &mut Vec<MessageName>,
    visiting: &mut HashSet<MessageName>,
    report: &mut CheckReport,
) {
    for node in nodes {
        match node {
            MessageNode::MessageRef { name, .. } => {
                if let Some(def) = doc.messages.get(name) {
                    detect_cycles(doc, name, def, stack, visiting, report);
                }
            }
            MessageNode::Select { cases, .. } => {
                for child_nodes in cases.values() {
                    walk_refs(doc, child_nodes, stack, visiting, report);
                }
            }
            MessageNode::Text(_) | MessageNode::Argument { .. } => {}
        }
    }
}

fn join_stack(stack: &[MessageName]) -> String {
    stack.iter().map(MessageName::as_str).collect::<Vec<_>>().join("->")
}

fn join_names(names: &BTreeSet<Arc<str>>) -> String {
    names.iter().map(|s| s.as_ref()).collect::<Vec<_>>().join(",")
}

fn collect_arg_schemas(doc: &LocalizationDocument) -> BTreeMap<MessageName, BTreeSet<Arc<str>>> {
    let mut out = BTreeMap::new();
    for (name, def) in &doc.messages {
        let mut args = BTreeSet::new();
        collect_args_from_def(def, &mut args);
        out.insert(name.clone(), args);
    }
    out
}

fn collect_args_from_def(def: &MessageDefinition, args: &mut BTreeSet<Arc<str>>) {
    match def {
        MessageDefinition::Text(_) => {}
        MessageDefinition::Pattern(nodes) => collect_args_from_nodes(nodes, args),
        MessageDefinition::Select { argument, cases, .. } => {
            args.insert(argument.clone());
            for nodes in cases.values() {
                collect_args_from_nodes(nodes, args);
            }
        }
    }
}

fn collect_args_from_nodes(nodes: &[MessageNode], args: &mut BTreeSet<Arc<str>>) {
    for node in nodes {
        match node {
            MessageNode::Argument { name, .. } => {
                args.insert(name.clone());
            }
            MessageNode::Select { argument, cases, .. } => {
                args.insert(argument.clone());
                for child in cases.values() {
                    collect_args_from_nodes(child, args);
                }
            }
            MessageNode::Text(_) | MessageNode::MessageRef { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        document::{MessageDefinition, MessageNode, SelectKind},
        locale::LocaleId,
    };
    use std::sync::Arc;

    #[test]
    fn detects_missing_other_case() {
        let mut doc = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
        let mut cases = BTreeMap::new();
        cases.insert(Arc::from("one"), vec![MessageNode::Text(Arc::from("one item"))]);
        doc.insert("item_count", MessageDefinition::Select { argument: Arc::from("count"), kind: SelectKind::Cardinal, cases });
        let report = check_document(&doc);
        assert!(report.issues.iter().any(|i| i.kind == CheckIssueKind::MissingSelectCase));
    }

    #[test]
    fn detects_missing_key_across_locales() {
        let mut en = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
        en.insert("menu.quit", MessageDefinition::Text(Arc::from("Quit")));
        let zh = LocalizationDocument::new(LocaleId::parse("zh-Hans-CN").unwrap(), "game");
        let report = check_locale_set(&en, &[zh]);
        assert!(report.issues.iter().any(|i| i.kind == CheckIssueKind::MissingKey));
    }

    #[test]
    fn issues_carry_tokens_not_prose() {
        let mut en = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
        en.insert("a", MessageDefinition::Text(Arc::from("A")));
        let mut zh = LocalizationDocument::new(LocaleId::parse("zh-Hans-CN").unwrap(), "other");
        zh.insert("a", MessageDefinition::Text(Arc::from("甲")));
        let report = check_locale_set(&en, &[zh]);
        let issue = report.issues.iter().find(|i| i.kind == CheckIssueKind::NamespaceMismatch).expect("namespace mismatch");
        let token = issue.token.as_deref().unwrap_or("");
        assert!(token.contains("!="));
        assert!(!token.contains(' '));
    }
}
