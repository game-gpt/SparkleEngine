//! Spark **Prefab**：声明式实体图、实例覆盖与嵌套。
//!
//! - 节点使用本地字符串 ID（非 UUID）
//! - 源文件用路径引用资源（[`spark_asset::AssetRef`]）
//! - 实例只存 override 补丁，不复制整树

#![warn(missing_docs)]

mod document;
mod error;
mod instance;
mod r#override;
mod validate;

pub use document::{PREFAB_SCHEMA, PREFAB_VERSION, PrefabDocument, PrefabNode};
pub use error::PrefabError;
pub use instance::PrefabInstance;
pub use r#override::{OverridePath, parse_override_path};
pub use validate::{nested_ref, validate_prefab_file};
