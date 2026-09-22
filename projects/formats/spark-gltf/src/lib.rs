//! glTF 2.0 → Spark CPU 骨架 / 剪辑 / 蒙皮网格。
//!
//! 约定：右手、Y-up、米、列主序。不含 GPU 上传与游戏 socket 强校验。

#![forbid(missing_docs)]
mod import;
mod mesh;
mod skin;

pub use import::{GltfAsset, GltfError, ImportedMesh, import_path, import_slice};
pub use spark_animator::{Skeleton, SkinnedAnimationClip};
pub use spark_renderer::SkinnedVertex;
