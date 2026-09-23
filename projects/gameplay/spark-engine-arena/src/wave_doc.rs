//! 波次 JSON 文档（数据驱动，不含敌种语义）。

use serde::Deserialize;
use spark_types::SparkError;

use crate::wave::{WaveEvent, WaveTimeline};

/// 波次文件根。
#[derive(Debug, Clone, Deserialize)]
pub struct WaveDocument {
    /// 模式版本，当前为 `1`。
    pub version: u32,
    /// 按秒触发的事件列表。
    pub events: Vec<WaveEventDef>,
}

/// 单条生成定义。
#[derive(Debug, Clone, Deserialize)]
pub struct WaveEventDef {
    /// 触发时刻（秒）。
    pub at_secs: f32,
    /// 游戏自定义种类 tag。
    pub tag: u32,
    /// 同刻重复次数，默认 `1`。
    #[serde(default = "default_count")]
    pub count: u32,
}

fn default_count() -> u32 {
    1
}

impl WaveDocument {
    /// 从 JSON 文本解析。
    pub fn from_json(text: &str) -> Result<Self, SparkError> {
        serde_json::from_str(text).map_err(|e| SparkError::invalid_argument("wave_json").caused_by(e))
    }

    /// 展开 `count` 并构建 [`WaveTimeline`]。
    pub fn into_timeline(self) -> WaveTimeline {
        let mut flat = Vec::new();
        for def in self.events {
            let n = def.count.max(1);
            for _ in 0..n {
                flat.push(WaveEvent { at_secs: def.at_secs, tag: def.tag });
            }
        }
        WaveTimeline::new(flat)
    }
}

/// 解析 JSON 并直接得到时间轴。
pub fn timeline_from_json(text: &str) -> Result<WaveTimeline, SparkError> {
    Ok(WaveDocument::from_json(text)?.into_timeline())
}
