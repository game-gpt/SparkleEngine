//! 扁平 VON 语言包 → [`LocalizationDocument`]。
//!
//! 与 JSON 落到同一中间模型。消息键使用 `message.` 前缀，避免和清单字段撞名。
//!
//! ```text
//! locale = "en"
//! namespace = "demo"
//! message.menu.quit = "Quit"
//! message.hello = "Hello, {name}"
//! ```

use std::sync::Arc;

use crate::{
    document::{LocalizationDocument, MessageDefinition},
    locale::{LocaleId, LocaleParseError},
    manifest_von::{ManifestVonError, parse_string, split_assign, strip_line_comment},
};

/// VON 语言包解析错误。
///
/// 稳定码见 [`Self::code`]；行号从 1 起，供编辑器定位。
#[derive(Debug)]
pub enum DocumentVonError {
    /// 复用清单侧 VON 词法/字符串错误。
    Manifest(ManifestVonError),
    /// `locale` 字段无法解析为 BCP 47。
    Locale(LocaleParseError),
    /// 缺少必填 `locale = ...`。
    MissingLocale,
    /// 缺少必填 `namespace = ...`。
    MissingNamespace,
    /// 非空行缺少 `=` 赋值。
    MissingAssign {
        /// 1-based 行号。
        line: usize,
    },
    /// 未知顶层字段（非 `locale` / `namespace` / `message.*`）。
    UnknownField {
        /// 原始字段名。
        field: String,
        /// 1-based 行号。
        line: usize,
    },
}

impl DocumentVonError {
    /// 稳定机器码；嵌套变体转发源码。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Manifest(e) => e.code(),
            Self::Locale(e) => e.code(),
            Self::MissingLocale => "spark.localization.von_doc_missing_locale",
            Self::MissingNamespace => "spark.localization.von_doc_missing_namespace",
            Self::MissingAssign { .. } => "spark.localization.von_missing_assign",
            Self::UnknownField { .. } => "spark.localization.von_unknown_field",
        }
    }

    /// 结构化参数（行号、字段名等）。
    pub fn args(&self) -> spark_types::ErrorArgs {
        use spark_types::{ErrorArg, ErrorArgs};
        match self {
            Self::Manifest(e) => e.args(),
            Self::Locale(e) => e.args(),
            Self::MissingLocale | Self::MissingNamespace => ErrorArgs::new(),
            Self::MissingAssign { line } => ErrorArgs::new().with("line", ErrorArg::Unsigned(*line as u64)),
            Self::UnknownField { field, line } => {
                ErrorArgs::new().with("field", ErrorArg::String(Arc::from(field.as_str()))).with("line", ErrorArg::Unsigned(*line as u64))
            }
        }
    }
}

impl std::fmt::Display for DocumentVonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for DocumentVonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Manifest(e) => Some(e),
            Self::Locale(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ManifestVonError> for DocumentVonError {
    fn from(value: ManifestVonError) -> Self {
        Self::Manifest(value)
    }
}

impl From<LocaleParseError> for DocumentVonError {
    fn from(value: LocaleParseError) -> Self {
        Self::Locale(value)
    }
}

/// 解析扁平 VON 语言包。
pub fn document_from_von_str(text: &str) -> Result<LocalizationDocument, DocumentVonError> {
    let mut locale: Option<LocaleId> = None;
    let mut namespace: Option<String> = None;
    let mut messages: Vec<(String, String)> = Vec::new();

    for (line_no, raw_line) in text.lines().enumerate() {
        let owned = strip_line_comment(raw_line);
        let line = owned.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = split_assign(line)
        else {
            return Err(DocumentVonError::MissingAssign { line: line_no + 1 });
        };
        if let Some(name) = key.strip_prefix("message.") {
            if name.is_empty() {
                return Err(DocumentVonError::UnknownField { field: key.to_string(), line: line_no + 1 });
            }
            messages.push((name.to_string(), parse_string(value)?));
            continue;
        }
        match key {
            "locale" => locale = Some(LocaleId::parse(&parse_string(value)?)?),
            "namespace" => namespace = Some(parse_string(value)?),
            other => {
                return Err(DocumentVonError::UnknownField { field: other.to_string(), line: line_no + 1 });
            }
        }
    }

    let locale = locale.ok_or(DocumentVonError::MissingLocale)?;
    let namespace = namespace.ok_or(DocumentVonError::MissingNamespace)?;
    let mut doc = LocalizationDocument::new(locale, namespace);
    for (name, text) in messages {
        doc.insert(name, MessageDefinition::from_template_sugar(&text));
    }
    Ok(doc)
}
