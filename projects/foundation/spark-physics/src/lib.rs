//! Spark **物理框架**（无游戏语义）。
//!
//! 2D：AABB 刚体、均匀网格宽相、圆/盒窄相委托 `spark-geometry`，以及带重力的固定步。
//! 3D：扫掠原语直接再导出 `spark-geometry`（[`aabb_sweep`] / [`aabb_sweep_resolve`]）。
//! **不**提供行星重力模式、方块碰撞表或玩法材质。

mod body;
mod broadphase;
mod world;

pub use body::{BodyId, BodyKind, Collider2, RigidBody2};
pub use broadphase::{Broadphase, UniformGrid};
pub use spark_geometry::{aabb_sweep, aabb_sweep_resolve, Aabb3, SweepHit, Vec3};
pub use world::{Contact, PhysicsConfig, PhysicsWorld};

use std::fmt;

use spark_core::SparkError;

/// 物理层结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum PhysicsError {
    Spark(SparkError),
    Internal { detail: String },
}

impl PhysicsError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Spark(_) => "spark.physics.spark",
            Self::Internal { .. } => "spark.physics.internal",
        }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal {
            detail: detail.into(),
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
