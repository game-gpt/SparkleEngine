//! VON 编辑计划。

use serde::{Deserialize, Serialize};

use crate::op::EditOp;

/// 可版本控制的编辑计划（过渡格式；Sparkle Script 将编译到同类操作）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditPlan {
    /// 可选说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 操作序列。
    #[serde(default)]
    pub ops: Vec<EditOp>,
}

impl EditPlan {
    /// 从 VON 文本解析。
    pub fn from_von(text: &str) -> Result<Self, String> {
        oak_von::from_str(text).map_err(|e| e.to_string())
    }
}
