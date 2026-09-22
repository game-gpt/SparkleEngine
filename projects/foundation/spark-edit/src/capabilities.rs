//! 编辑能力（权限门闩）。

use serde::{Deserialize, Serialize};

/// 会话能力集。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditCapabilities {
    /// 读项目 / 校验。
    pub read_project: bool,
    /// 写资源、旁车、Prefab。
    pub write_assets: bool,
}

impl EditCapabilities {
    /// 默认只读（MCP / 远程 Agent）。
    pub fn read_only() -> Self {
        Self { read_project: true, write_assets: false }
    }

    /// 本地项目编辑（可 apply）。
    pub fn project_edit() -> Self {
        Self { read_project: true, write_assets: true }
    }

    /// 从 CLI / MCP 字符串列表解析。
    ///
    /// 接受：`read-project`、`read-only`、`write-assets`、`project-edit`。
    pub fn from_tokens<I, S>(tokens: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut caps = Self { read_project: false, write_assets: false };
        for raw in tokens {
            match raw.as_ref().trim().to_ascii_lowercase().as_str() {
                "read-project" | "read_project" | "read-only" | "readonly" => {
                    caps.read_project = true;
                }
                "write-assets" | "write_assets" => {
                    caps.read_project = true;
                    caps.write_assets = true;
                }
                "project-edit" | "project_edit" => {
                    caps.read_project = true;
                    caps.write_assets = true;
                }
                _ => {}
            }
        }
        if !caps.read_project && !caps.write_assets {
            return Self::read_only();
        }
        caps
    }

    /// 是否允许 `apply` 写盘。
    pub fn allows_apply(self) -> bool {
        self.write_assets
    }
}

impl Default for EditCapabilities {
    fn default() -> Self {
        Self::project_edit()
    }
}
