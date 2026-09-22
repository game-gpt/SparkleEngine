//! BCP 47 Locale 标识与确定性协商。

use std::{fmt, sync::Arc};

/// 书写方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextDirection {
    /// 从左到右（多数拉丁与 CJK 界面默认）。
    #[default]
    Ltr,
    /// 从右到左（如阿拉伯文、希伯来文）。
    Rtl,
}

impl TextDirection {
    /// 按 Unicode 脚本粗判默认方向；未知脚本回退 LTR。
    ///
    /// 完整方向传播仍应由消息包与布局层声明；此处仅作协商缺省。
    pub fn guess_from_language(language: &str) -> Self {
        match language {
            "ar" | "he" | "fa" | "ur" | "ps" | "yi" => Self::Rtl,
            _ => Self::Ltr,
        }
    }
}

/// Locale 解析错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocaleParseError {
    /// 空标签。
    Empty,
    /// 标签无法规范化。`detail` 是校验器原始说明，不是用户文案。
    Invalid { detail: String },
}

impl LocaleParseError {
    pub fn invalid(detail: impl Into<String>) -> Self {
        Self::Invalid { detail: detail.into() }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Empty => "spark.localization.locale_empty",
            Self::Invalid { .. } => "spark.localization.locale_invalid",
        }
    }

    pub fn args(&self) -> spark_types::ErrorArgs {
        use spark_types::{ErrorArg, ErrorArgs};
        use std::sync::Arc;
        match self {
            Self::Empty => ErrorArgs::new(),
            Self::Invalid { detail } => ErrorArgs::new().with("reason", ErrorArg::String(Arc::from(detail.as_str()))),
        }
    }
}

impl std::fmt::Display for LocaleParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for LocaleParseError {}

/// 经过验证与规范化的 BCP 47 Locale。
///
/// 内部按小写 language、首字母大写 script、大写 region 存储；
/// 显示用 [`LocaleId::as_str`] 输出规范标签（如 `zh-Hans-CN`）。
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocaleId {
    language: Arc<str>,
    script: Option<Arc<str>>,
    region: Option<Arc<str>>,
    /// 规范化后的完整标签，供比较与日志复用。
    tag: Arc<str>,
}

impl fmt::Debug for LocaleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LocaleId").field(&self.tag).finish()
    }
}

impl fmt::Display for LocaleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.tag)
    }
}

impl LocaleId {
    /// 引擎保证存在的根回退（英文基线）。
    pub fn root_fallback() -> Self {
        Self::parse("en").expect("root fallback locale is valid")
    }

    /// 解析 BCP 47 标签。忽略 Unicode 扩展（`u-…`）与私有扩展（`x-…`）。
    pub fn parse(input: &str) -> Result<Self, LocaleParseError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(LocaleParseError::Empty);
        }

        let mut language: Option<String> = None;
        let mut script: Option<String> = None;
        let mut region: Option<String> = None;

        for (index, raw) in trimmed.split(['-', '_']).enumerate() {
            if raw.is_empty() {
                return Err(LocaleParseError::invalid(trimmed));
            }
            // 扩展子标签：后面整段丢弃。
            if raw.eq_ignore_ascii_case("u") || raw.eq_ignore_ascii_case("x") || raw.eq_ignore_ascii_case("t") {
                break;
            }

            match index {
                0 => {
                    if !(2..=8).contains(&raw.len()) || !raw.chars().all(|c| c.is_ascii_alphabetic()) {
                        return Err(LocaleParseError::invalid(trimmed));
                    }
                    language = Some(raw.to_ascii_lowercase());
                }
                1 => {
                    if raw.len() == 4 && raw.chars().all(|c| c.is_ascii_alphabetic()) {
                        let mut s = raw.to_ascii_lowercase();
                        if let Some(first) = s.get_mut(0..1) {
                            first.make_ascii_uppercase();
                        }
                        script = Some(s);
                    }
                    else if is_region(raw) {
                        region = Some(normalize_region(raw));
                    }
                    else {
                        // 变体等：协商链会去掉，解析时忽略后续。
                        break;
                    }
                }
                2 => {
                    if region.is_none() && is_region(raw) {
                        region = Some(normalize_region(raw));
                    }
                    else {
                        break;
                    }
                }
                _ => break,
            }
        }

        let language = language.ok_or_else(|| LocaleParseError::invalid(trimmed))?;
        Ok(Self::from_parts(language, script, region))
    }

    fn from_parts(language: String, script: Option<String>, region: Option<String>) -> Self {
        let mut tag = language.clone();
        if let Some(script) = script.as_ref() {
            tag.push('-');
            tag.push_str(script);
        }
        if let Some(region) = region.as_ref() {
            tag.push('-');
            tag.push_str(region);
        }
        Self { language: Arc::from(language), script: script.map(Arc::from), region: region.map(Arc::from), tag: Arc::from(tag) }
    }

    pub fn as_str(&self) -> &str {
        &self.tag
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    pub fn direction(&self) -> TextDirection {
        TextDirection::guess_from_language(self.language())
    }

    /// 生成确定性回退候选（不含产品默认与引擎根，由协商器追加）。
    ///
    /// 顺序：精确 → 语言+文字+地区 → 语言+文字 → 语言+地区 → 语言。
    pub fn fallback_candidates(&self) -> Vec<LocaleId> {
        let mut out = Vec::with_capacity(5);
        let push_unique = |list: &mut Vec<LocaleId>, locale: LocaleId| {
            if !list.iter().any(|existing| existing == &locale) {
                list.push(locale);
            }
        };

        push_unique(&mut out, self.clone());

        if self.script.is_some() && self.region.is_some() {
            push_unique(
                &mut out,
                Self::from_parts(
                    self.language.to_string(),
                    self.script.as_ref().map(|s| s.to_string()),
                    self.region.as_ref().map(|s| s.to_string()),
                ),
            );
        }

        if let Some(script) = self.script.as_ref() {
            push_unique(&mut out, Self::from_parts(self.language.to_string(), Some(script.to_string()), None));
        }

        if let Some(region) = self.region.as_ref() {
            push_unique(&mut out, Self::from_parts(self.language.to_string(), None, Some(region.to_string())));
        }

        push_unique(&mut out, Self::from_parts(self.language.to_string(), None, None));

        out
    }
}

fn is_region(raw: &str) -> bool {
    (raw.len() == 2 && raw.chars().all(|c| c.is_ascii_alphabetic())) || (raw.len() == 3 && raw.chars().all(|c| c.is_ascii_digit()))
}

fn normalize_region(raw: &str) -> String {
    if raw.chars().all(|c| c.is_ascii_alphabetic()) { raw.to_ascii_uppercase() } else { raw.to_string() }
}

/// 用户偏好与产品回退配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleRequest {
    pub preferred: Vec<LocaleId>,
    pub fallback: Vec<LocaleId>,
}

impl LocaleRequest {
    pub fn new(preferred: Vec<LocaleId>, fallback: Vec<LocaleId>) -> Self {
        Self { preferred, fallback }
    }
}

/// 在已安装 Locale 集合上确定性协商。
///
/// 对每个偏好按 [`LocaleId::fallback_candidates`] 展开，再追加请求级 fallback、
/// `product_default` 与引擎根回退。返回第一个命中 `available` 的 Locale。
pub fn negotiate(request: &LocaleRequest, available: &[LocaleId], product_default: &LocaleId) -> LocaleId {
    let mut chain: Vec<LocaleId> = Vec::new();
    let push_unique = |list: &mut Vec<LocaleId>, locale: LocaleId| {
        if !list.iter().any(|existing| existing == &locale) {
            list.push(locale);
        }
    };

    for preferred in &request.preferred {
        for candidate in preferred.fallback_candidates() {
            push_unique(&mut chain, candidate);
        }
    }
    for fallback in &request.fallback {
        for candidate in fallback.fallback_candidates() {
            push_unique(&mut chain, candidate);
        }
    }
    for candidate in product_default.fallback_candidates() {
        push_unique(&mut chain, candidate);
    }
    push_unique(&mut chain, LocaleId::root_fallback());

    for candidate in &chain {
        if available.iter().any(|item| item == candidate) {
            return candidate.clone();
        }
    }

    // available 为空或未覆盖根回退时，仍返回确定性结果。
    if let Some(first) = available.first() {
        return first.clone();
    }
    product_default.clone()
}

/// 展开完整 fallback 链（已协商 Locale → 产品默认 → 根），供快照持有。
pub fn build_fallback_chain(resolved: &LocaleId, product_default: &LocaleId, available: &[LocaleId]) -> Arc<[LocaleId]> {
    let mut chain: Vec<LocaleId> = Vec::new();
    let push_if_available = |list: &mut Vec<LocaleId>, locale: LocaleId| {
        if available.iter().any(|item| item == &locale) && !list.iter().any(|existing| existing == &locale) {
            list.push(locale);
        }
    };

    for candidate in resolved.fallback_candidates() {
        push_if_available(&mut chain, candidate);
    }
    for candidate in product_default.fallback_candidates() {
        push_if_available(&mut chain, candidate);
    }
    push_if_available(&mut chain, LocaleId::root_fallback());

    if chain.is_empty() {
        if let Some(first) = available.first() {
            chain.push(first.clone());
        }
        else {
            chain.push(resolved.clone());
        }
    }

    Arc::from(chain)
}
