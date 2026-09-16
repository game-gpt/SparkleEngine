//! Spark **2D 物理框架**（无游戏语义）。
//!
//! 提供 AABB 刚体、均匀网格宽相、圆/盒窄相委托 `spark-geometry`，以及带重力的固定步。
//! **不**提供行星重力模式、方块碰撞表或玩法材质。

mod body;
mod broadphase;
mod world;

pub use body::{BodyId, BodyKind, Collider2, RigidBody2};
pub use broadphase::{Broadphase, UniformGrid};
pub use world::{Contact, PhysicsConfig, PhysicsWorld};

use spark_core::SparkError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PhysicsError {
    #[error(transparent)]
    Spark(#[from] SparkError),
    #[error("{0}")]
    Message(String),
}
