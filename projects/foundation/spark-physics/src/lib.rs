//! Spark **物理框架**（无游戏语义）。
//!
//! 2D：AABB 刚体、均匀网格宽相、圆/盒窄相委托 `spark-geometry`，以及带重力的固定步。
//! 3D：扫掠原语直接再导出 `spark-geometry`（[`aabb_sweep`] / [`aabb_sweep_resolve`]）。
//! **不**提供行星重力模式、方块碰撞表或玩法材质。

#![forbid(missing_docs)]
mod body;
mod broadphase;
mod world;

pub use body::{BodyId, BodyKind, Collider2, RigidBody2};
pub use broadphase::{Broadphase, UniformGrid};
pub use spark_geometry::{Aabb3, SweepHit, Vec2, Vec3, aabb_sweep, aabb_sweep_resolve};
pub use world::{Contact, PhysicsConfig, PhysicsWorld};

use std::{fmt, sync::Arc};

use spark_types::{ErrorArg, ErrorArgs, SparkError};

/// 物理层结构化错误。`Display` 只输出稳定码。
///
/// [`Self::Spark`] 透传底层 [`SparkError`] 的显示；[`Self::Internal`] 只暴露稳定码
/// `spark.physics.internal`，细节进 [`Self::args`]。
#[derive(Debug)]
pub enum PhysicsError {
    /// 透传的底层 Spark 错误。
    Spark(SparkError),
    /// `detail` 必须是机器令牌，不是自然语言。
    Internal {
        /// 机器可读原因令牌（写入 `args.reason`）。
        detail: String,
    },
}

impl PhysicsError {
    /// 稳定错误码（`spark.physics.*`）。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Spark(_) => "spark.physics.spark",
            Self::Internal { .. } => "spark.physics.internal",
        }
    }

    /// 构造内部错误；`detail` 应为机器令牌。
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal { detail: detail.into() }
    }

    /// 结构化参数：`Spark` 克隆其 `args`；`Internal` 写入 `reason`。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Spark(e) => e.args.clone(),
            Self::Internal { detail } => ErrorArgs::new().with("reason", ErrorArg::String(Arc::from(detail.as_str()))),
        }
    }
}

impl fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spark(e) => e.fmt(f),
            other => f.write_str(other.code()),
        }
    }
}

impl std::error::Error for PhysicsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spark(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SparkError> for PhysicsError {
    fn from(value: SparkError) -> Self {
        Self::Spark(value)
    }
}
