//! Spark **竞技场**特异化引擎壳。
//!
//! 双摇杆生存类游戏的通用池、波次与空间查询；**禁止**敌种表、Boss 剧本与 Meta 成长。

#![forbid(missing_docs)]
mod clock;
mod pool;
mod spatial;
mod wave;
mod wave_doc;

pub use clock::RunClock;
pub use pool::{BodyId, KineticBody, KineticPool};
pub use spatial::{CircleGrid, circles_hit};
pub use wave::{WaveEvent, WaveTimeline};
pub use wave_doc::{WaveDocument, WaveEventDef, timeline_from_json};
