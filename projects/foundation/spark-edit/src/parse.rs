//! 解析 VON：优先 `ops` 计划；含 `calls` / `profile` 时按 Edit profile 宿主脚本降级。

use crate::host_api::{EDIT_PROFILE_ID, EditHostScript};
use crate::plan::EditPlan;

/// 解析编辑源文本。
pub fn parse_edit_source(text: &str) -> Result<EditPlan, String> {
    let looks_like_host = text.contains("calls") || text.contains("profile");
    if looks_like_host {
        if let Ok(script) = EditHostScript::from_von(text) {
            if !script.calls.is_empty() || script.profile.as_deref() == Some(EDIT_PROFILE_ID) {
                return script.lower();
            }
        }
    }
    EditPlan::from_von(text)
}
