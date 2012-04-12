//! Spark **导体图框架**（无游戏语义）。
//!
//! 提供节点、无向边、通道与多源可达查询。不包含飞船引擎、导线方块表或红石花活。

mod graph;

pub use graph::{Channel, CircuitError, CircuitGraph, NodeId};
