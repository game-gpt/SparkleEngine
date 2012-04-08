//! 模组清单（`mod.toml`）。

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use spark_core::SparkError;

use crate::EngineError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModManifest {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    /// 相对模组根的入口脚本；缺省则只挂载资源 / 清单。
    #[serde(default)]
    pub entry: Option<String>,
    /// 脚本语言：`valkyrie` / `lua` / `ruby`；缺省时按入口扩展名推断。
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

fn default_version() -> String {
    "0.0.0".into()
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
        let mut m: Self = toml::from_str(&text).map_err(|e| {
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
        Self::from_path(dir.as_ref().join("mod.toml"))
    }
}
