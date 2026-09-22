//! Prefab 错误（稳定码；`Display` 不输出自然语言）。

use std::{fmt, path::PathBuf, sync::Arc};

use spark_types::{ErrorArg, ErrorArgs};

/// Prefab 文档 / 实例错误。
#[derive(Debug)]
pub enum PrefabError {
    /// IO 失败。
    Io {
        /// 相关路径。
        path: PathBuf,
        /// 细节。
        detail: String,
    },
    /// JSON 解析失败。
    Parse {
        /// 相关路径或 `<memory>`。
        path: PathBuf,
        /// 细节。
        detail: String,
    },
    /// `schema` 不是 `spark.prefab`。
    BadSchema {
        /// 实际值。
        found: String,
    },
    /// 根节点 ID 不在 `nodes` 中。
    RootMissing {
        /// 声明的根 ID。
        root: String,
    },
    /// 引用了不存在的节点。
    NodeMissing {
        /// 缺失节点 ID。
        node: String,
    },
    /// `children` 引用了未知节点。
    ChildMissing {
        /// 父节点。
        parent: String,
        /// 缺失子节点。
        child: String,
    },
    /// 节点图存在环（同文档内父子）。
    NodeCycle {
        /// 参与环的节点。
        node: String,
    },
    /// 节点 ID 非法（空或含 `/`）。
    BadNodeId {
        /// 非法 ID。
        node: String,
    },
    /// 嵌套 Prefab 形成环。
    PrefabCycle {
        /// 环上的 Prefab 路径。
        path: String,
    },
    /// 覆盖目标节点路径不存在。
    OverrideTargetMissing {
        /// 覆盖键。
        target: String,
    },
    /// 覆盖路径语法无效。
    BadOverridePath {
        /// 原始覆盖键。
        target: String,
    },
}

impl PrefabError {
    /// 稳定错误码。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "spark.prefab.io",
            Self::Parse { .. } => "spark.prefab.parse",
            Self::BadSchema { .. } => "spark.prefab.bad_schema",
            Self::RootMissing { .. } => "spark.prefab.root_missing",
            Self::NodeMissing { .. } => "spark.prefab.node_missing",
            Self::ChildMissing { .. } => "spark.prefab.child_missing",
            Self::NodeCycle { .. } => "spark.prefab.node_cycle",
            Self::BadNodeId { .. } => "spark.prefab.bad_node_id",
            Self::PrefabCycle { .. } => "spark.prefab.prefab_cycle",
            Self::OverrideTargetMissing { .. } => "spark.prefab.override_target_missing",
            Self::BadOverridePath { .. } => "spark.prefab.bad_override_path",
        }
    }

    /// 类型化参数。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Io { path, detail } | Self::Parse { path, detail } => ErrorArgs::new()
                .with("path", ErrorArg::String(Arc::from(path.to_string_lossy().as_ref())))
                .with("detail", ErrorArg::String(Arc::from(detail.as_str()))),
            Self::BadSchema { found } => ErrorArgs::new().with("found", ErrorArg::String(Arc::from(found.as_str()))),
            Self::RootMissing { root } => ErrorArgs::new().with("root", ErrorArg::String(Arc::from(root.as_str()))),
            Self::NodeMissing { node } => ErrorArgs::new().with("node", ErrorArg::String(Arc::from(node.as_str()))),
            Self::ChildMissing { parent, child } => ErrorArgs::new()
                .with("parent", ErrorArg::String(Arc::from(parent.as_str())))
                .with("child", ErrorArg::String(Arc::from(child.as_str()))),
            Self::NodeCycle { node } | Self::BadNodeId { node } => {
                ErrorArgs::new().with("node", ErrorArg::String(Arc::from(node.as_str())))
            }
            Self::PrefabCycle { path } => ErrorArgs::new().with("path", ErrorArg::String(Arc::from(path.as_str()))),
            Self::OverrideTargetMissing { target } | Self::BadOverridePath { target } => {
                ErrorArgs::new().with("target", ErrorArg::String(Arc::from(target.as_str())))
            }
        }
    }
}

impl fmt::Display for PrefabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for PrefabError {}
