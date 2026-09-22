//! `localization.von` 扁平清单解析。
//!
//! 完整嵌套 VON（对象/分片表）后续接 `oak-von`；此处覆盖模组级常用顶层字段，
//! 与 `spark-engine` 的 `mod.von` 解析风格一致。

use std::sync::Arc;

use crate::{
    locale::{LocaleId, LocaleParseError},
    manifest::{LocaleEntry, LocalizationManifest, NamespaceOwner},
    message::NamespaceId,
};

/// VON 清单解析错误。
#[derive(Debug)]
pub enum ManifestVonError {
    Locale(LocaleParseError),
    MissingAssign { line: usize },
    UnknownField { field: String, line: usize },
    NamespaceArity,
    MissingProductDefault,
    ExpectedString,
    ExpectedInteger,
    ExpectedArray,
}

impl ManifestVonError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Locale(e) => e.code(),
            Self::MissingAssign { .. } => "spark.localization.von_missing_assign",
            Self::UnknownField { .. } => "spark.localization.von_unknown_field",
            Self::NamespaceArity => "spark.localization.von_namespace_arity",
            Self::MissingProductDefault => "spark.localization.von_missing_product_default",
            Self::ExpectedString => "spark.localization.von_expected_string",
            Self::ExpectedInteger => "spark.localization.von_expected_integer",
            Self::ExpectedArray => "spark.localization.von_expected_array",
        }
    }

    pub fn args(&self) -> spark_core::ErrorArgs {
        use spark_core::{ErrorArg, ErrorArgs};
        match self {
            Self::Locale(e) => e.args(),
            Self::MissingAssign { line } => ErrorArgs::new().with("line", ErrorArg::Unsigned(*line as u64)),
            Self::UnknownField { field, line } => {
                ErrorArgs::new().with("field", ErrorArg::String(Arc::from(field.as_str()))).with("line", ErrorArg::Unsigned(*line as u64))
            }
            Self::NamespaceArity | Self::MissingProductDefault | Self::ExpectedString | Self::ExpectedInteger | Self::ExpectedArray => {
                ErrorArgs::new()
            }
        }
    }
}

impl std::fmt::Display for ManifestVonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ManifestVonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Locale(e) => Some(e),
            _ => None,
        }
    }
}

impl From<LocaleParseError> for ManifestVonError {
    fn from(value: LocaleParseError) -> Self {
        Self::Locale(value)
    }
}

/// 解析扁平 `localization.von`。
///
/// 支持字段：
/// - `product_default = "en"`
/// - `cldr_version = "..."`
/// - `format_version = 1`
/// - `locales = ["en", "zh-Hans"]`
/// - `bundled = ["en"]`（缺省则全部 bundled）
pub fn manifest_from_von_str(text: &str) -> Result<LocalizationManifest, ManifestVonError> {
    let mut product_default: Option<LocaleId> = None;
    let mut cldr_version: Option<Arc<str>> = None;
    let mut format_version: Option<u32> = None;
    let mut locales: Vec<LocaleId> = Vec::new();
    let mut bundled: Option<Vec<LocaleId>> = None;
    let mut namespaces: Vec<(String, String, bool)> = Vec::new();
    let mut shard_files: Vec<String> = Vec::new();

    for (line_no, raw_line) in text.lines().enumerate() {
        let owned = strip_line_comment(raw_line);
        let line = owned.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = split_assign(line)
        else {
            return Err(ManifestVonError::MissingAssign { line: line_no + 1 });
        };
        match key {
            "product_default" => {
                product_default = Some(LocaleId::parse(&parse_string(value)?)?);
            }
            "cldr_version" => cldr_version = Some(Arc::from(parse_string(value)?)),
            "format_version" => {
                format_version = Some(parse_u32(value)?);
            }
            "locales" => {
                locales = parse_string_array(value)?.into_iter().map(|s| LocaleId::parse(&s)).collect::<Result<Vec<_>, _>>()?;
            }
            "bundled" => {
                bundled = Some(parse_string_array(value)?.into_iter().map(|s| LocaleId::parse(&s)).collect::<Result<Vec<_>, _>>()?);
            }
            "namespace" => {
                // namespace = ["astracraft", "AstraCraft", false]
                let parts = parse_mixed_array(value)?;
                if parts.len() < 2 {
                    return Err(ManifestVonError::NamespaceArity);
                }
                let allow = parts.get(2).map(|p| p == "true").unwrap_or(false);
                namespaces.push((parts[0].clone(), parts[1].clone(), allow));
            }
            "shards" => {
                shard_files = parse_string_array(value)?;
            }
            other => {
                return Err(ManifestVonError::UnknownField { field: other.to_string(), line: line_no + 1 });
            }
        }
    }

    let product_default = product_default.ok_or(ManifestVonError::MissingProductDefault)?;

    let mut manifest = LocalizationManifest::new(product_default.clone());
    if let Some(v) = cldr_version {
        manifest.cldr_version = v;
    }
    if let Some(v) = format_version {
        manifest.format_version = v;
    }

    let bundled_set = bundled.unwrap_or_else(|| locales.clone());
    for locale in locales {
        let is_bundled = bundled_set.iter().any(|b| b == &locale);
        manifest.locales.push(LocaleEntry { locale, fallback: vec![], bundled: is_bundled, font_hint: None });
    }
    if manifest.locales.is_empty() {
        manifest.locales.push(LocaleEntry { locale: product_default, fallback: vec![], bundled: true, font_hint: None });
    }

    for (ns, owner, allow) in namespaces {
        manifest.namespaces.push(NamespaceOwner { namespace: NamespaceId::new(ns), owner: Arc::from(owner), allow_override: allow });
    }

    if !shard_files.is_empty() {
        manifest.shards.insert(manifest.product_default.clone(), shard_files.into_iter().map(Arc::from).collect());
    }

    Ok(manifest)
}

pub(crate) fn strip_line_comment(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut quote = '\0';
    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
                continue;
            }
            if ch == quote {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                in_string = true;
                quote = ch;
                out.push(ch);
            }
            '#' => break,
            '/' if chars.peek() == Some(&'/') => break,
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn split_assign(line: &str) -> Option<(&str, &str)> {
    let idx = line.find('=')?;
    let key = line[..idx].trim();
    let value = line[idx + 1..].trim();
    if key.is_empty() { None } else { Some((key, value)) }
}

pub(crate) fn parse_string(value: &str) -> Result<String, ManifestVonError> {
    let value = value.trim();
    if (value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')) {
        return Ok(value[1..value.len() - 1].to_string());
    }
    if value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        return Ok(value.to_string());
    }
    Err(ManifestVonError::ExpectedString)
}

fn parse_u32(value: &str) -> Result<u32, ManifestVonError> {
    value.trim().parse().map_err(|_| ManifestVonError::ExpectedInteger)
}

fn parse_string_array(value: &str) -> Result<Vec<String>, ManifestVonError> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(ManifestVonError::ExpectedArray);
    }
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in split_commas(inner) {
        out.push(parse_string(part.trim())?);
    }
    Ok(out)
}

fn parse_mixed_array(value: &str) -> Result<Vec<String>, ManifestVonError> {
    parse_string_array(value).or_else(|_| {
        // 允许 [astracraft, AstraCraft, false] 中裸标识
        let value = value.trim();
        if !value.starts_with('[') || !value.ends_with(']') {
            return Err(ManifestVonError::ExpectedArray);
        }
        let inner = value[1..value.len() - 1].trim();
        Ok(split_commas(inner)
            .into_iter()
            .map(|p| p.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect())
    })
}

fn split_commas(input: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_string = false;
    let mut quote = '\0';
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_string {
            if ch == '\\' {
                i += 2;
                continue;
            }
            if ch == quote {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match ch {
            '"' | '\'' => {
                in_string = true;
                quote = ch;
                i += 1;
            }
            ',' => {
                out.push(&input[start..i]);
                start = i + 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    out.push(&input[start..]);
    out
}
