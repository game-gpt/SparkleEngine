//! 将结构化诊断渲染为 [`LocalizedText`]（用户 / 调试 Locale）。

use std::sync::Arc;

use spark_core::{Diagnostic, ErrorArg, ErrorArgs, MessageKey, SparkError};

use crate::{
    message::{MessageArgs, MessageId, MessageRef, MessageValue, NamespaceId},
    snapshot::LocaleSnapshot,
    text::LocalizedText,
};

impl From<&MessageKey> for MessageRef {
    fn from(value: &MessageKey) -> Self {
        MessageRef { namespace: NamespaceId::new(value.namespace.as_ref()), message: MessageId::name(value.message.as_ref()) }
    }
}

/// 将诊断参数转为本地化消息参数（保留类型，不预拼句子）。
pub fn message_args_from_error_args(args: &ErrorArgs) -> MessageArgs {
    let mut out = MessageArgs::new();
    for (name, value) in args.iter() {
        out.insert(name, error_arg_to_message_value(value));
    }
    out
}

fn error_arg_to_message_value(value: &ErrorArg) -> MessageValue {
    match value {
        ErrorArg::Integer(v) => MessageValue::Integer(*v),
        ErrorArg::Unsigned(v) => {
            if *v <= i64::MAX as u64 {
                MessageValue::Integer(*v as i64)
            }
            else {
                MessageValue::String(Arc::from(v.to_string()))
            }
        }
        ErrorArg::Float(v) => MessageValue::String(Arc::from(v.to_string())),
        ErrorArg::String(s) | ErrorArg::AssetKey(s) | ErrorArg::Path(s) | ErrorArg::TypeName(s) => MessageValue::String(s.clone()),
        ErrorArg::Opcode(op) => MessageValue::Integer(i64::from(*op)),
        ErrorArg::EntityBits(bits) => {
            if *bits <= i64::MAX as u64 {
                MessageValue::Integer(*bits as i64)
            }
            else {
                MessageValue::String(Arc::from(bits.to_string()))
            }
        }
        ErrorArg::Bool(b) => MessageValue::Select(Arc::from(if *b { "true" } else { "false" })),
        ErrorArg::Span(span) => MessageValue::String(Arc::from(format!("{}..{}", span.start, span.end))),
    }
}

/// 按快照 Locale 渲染诊断主消息。
pub fn render_diagnostic(snapshot: &LocaleSnapshot, diagnostic: &Diagnostic) -> LocalizedText {
    let message = MessageRef::from(&diagnostic.message);
    let args = message_args_from_error_args(&diagnostic.args);
    snapshot.format(&message, &args)
}

/// 渲染错误事实对应的用户消息（经 [`Diagnostic::from_error`]）。
pub fn render_error(snapshot: &LocaleSnapshot, error: &SparkError) -> LocalizedText {
    render_diagnostic(snapshot, &Diagnostic::from_error(error.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compile::{CompileOptions, compile_documents},
        document::{LocalizationDocument, MessageDefinition},
        locale::LocaleId,
        snapshot::LocaleSnapshot,
    };
    use spark_core::{SparkError, codes};
    use std::sync::Arc;

    #[test]
    fn renders_user_message_from_error_code() {
        let mut doc = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "spark");
        doc.insert("error.asset.not_found", MessageDefinition::Text(Arc::from("Asset not found: {key}")));
        let bundle = compile_documents(&[doc], CompileOptions::default()).unwrap().bundle;
        let snap = LocaleSnapshot::from_bundle(
            LocaleId::parse("en").unwrap(),
            &LocaleId::parse("en").unwrap(),
            &[LocaleId::parse("en").unwrap()],
            1,
            bundle,
        );
        let err = SparkError::new(codes::asset_not_found()).arg("key", ErrorArg::AssetKey(Arc::from("textures/dirt.png")));
        let text = render_error(&snap, &err);
        assert_eq!(text.text.as_ref(), "Asset not found: textures/dirt.png");
        assert!(!text.text.contains("无法"));
    }
}
