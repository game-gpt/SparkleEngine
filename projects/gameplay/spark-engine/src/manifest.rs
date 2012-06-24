//! 模组清单（`mod.von`）。
//!
//! 作者配置使用 Oak VON 风格的键值文档；不使用 TOML。

use std::fs;
use std::path::Path;

use spark_core::SparkError;

use crate::EngineError;

/// 模组清单。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    /// 相对模组根的入口脚本；缺省则只挂载资源 / 清单。
    pub entry: Option<String>,
    /// 脚本语言：`valkyrie` / `lua` / `ruby`；缺省时按入口扩展名推断。
    pub language: Option<String>,
    pub dependencies: Vec<String>,
}

impl ModManifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|e| {
            EngineError::Spark(SparkError::Message(format!(
                "读取清单失败 {}: {e}",
                path.display()
            )))
        })?;
        let mut m = parse_mod_von(&text).map_err(|e| {
            EngineError::Message(format!("解析清单失败 {}: {e}", path.display()))
        })?;
        if m.name.is_empty() {
            m.name = m.id.clone();
        }
        if m.id.is_empty() {
            return Err(EngineError::Message(format!(
                "清单缺少 id：{}",
                path.display()
            )));
        }
        Ok(m)
    }

    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, EngineError> {
        Self::from_path(dir.as_ref().join("mod.von"))
    }
}

/// 解析扁平 `mod.von`（顶层 `key = value`）。
pub fn parse_mod_von(text: &str) -> Result<ModManifest, String> {
    let mut id = String::new();
    let mut name = String::new();
    let mut version = "0.0.0".to_string();
    let mut entry = None;
    let mut language = None;
    let mut dependencies = Vec::new();

    for (line_no, raw_line) in text.lines().enumerate() {
        let owned = strip_line_comment(raw_line);
        let line = owned.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = split_assign(line) else {
            return Err(format!("第 {} 行缺少 `=`：{raw_line}", line_no + 1));
        };
        match key {
            "id" => id = parse_string(value)?,
            "name" => name = parse_string(value)?,
            "version" => version = parse_string(value)?,
            "entry" => entry = Some(parse_string(value)?),
            "language" => language = Some(parse_string(value)?),
            "dependencies" => dependencies = parse_string_array(value)?,
            other => {
                return Err(format!("未知清单字段 `{other}`（第 {} 行）", line_no + 1));
            }
        }
    }

    Ok(ModManifest {
        id,
        name,
        version,
        entry,
        language,
        dependencies,
    })
}

fn strip_line_comment(line: &str) -> String {
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

fn split_assign(line: &str) -> Option<(&str, &str)> {
    let idx = line.find('=')?;
    let key = line[..idx].trim();
    let value = line[idx + 1..].trim();
    if key.is_empty() {
        return None;
    }
    Some((key, value))
}

fn parse_string(value: &str) -> Result<String, String> {
    let value = value.trim();
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        return Ok(unescape(&value[1..value.len() - 1]));
    }
    // 允许无空格裸标识（少见）。
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Ok(value.to_string());
    }
    Err(format!("期望字符串：{value}"))
}

fn parse_string_array(value: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(format!("期望字符串数组：{value}"));
    }
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for part in split_top_level_commas(inner) {
        items.push(parse_string(part.trim())?);
    }
    Ok(items)
}

fn split_top_level_commas(input: &str) -> Vec<&str> {
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

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('\'') => out.push('\''),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_mod_von() {
        let raw = r#"
# AstraCraft 原版
id = "demo"
name = "Demo"
version = "0.1.0"
entry = "main.vk"
language = "valkyrie"
dependencies = ["core", "extra"]
"#;
        let m = parse_mod_von(raw).unwrap();
        assert_eq!(m.id, "demo");
        assert_eq!(m.name, "Demo");
        assert_eq!(m.entry.as_deref(), Some("main.vk"));
        assert_eq!(m.dependencies, vec!["core", "extra"]);
    }
}
