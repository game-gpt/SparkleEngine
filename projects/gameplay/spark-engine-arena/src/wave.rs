//! 按秒触发的生成事件（只输出 tag，不解释语义）。

/// 一条待触发生成。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveEvent {
    /// 触发时刻（秒，相对 RunClock）。
    pub at_secs: f32,
    /// 游戏自定义种类 ID。
    pub tag: u32,
}

/// 有序事件队列；[`WaveTimeline::drain_due`] 每帧消费到期项。
#[derive(Debug, Clone)]
pub struct WaveTimeline {
    events: Vec<WaveEvent>,
    cursor: usize,
}

impl WaveTimeline {
    /// 构造并按时刻排序。
    pub fn new(mut events: Vec<WaveEvent>) -> Self {
        events.sort_by(|a, b| a.at_secs.partial_cmp(&b.at_secs).unwrap_or(std::cmp::Ordering::Equal));
        Self { events, cursor: 0 }
    }

    /// 是否已全部触发。
    pub fn finished(&self) -> bool {
        self.cursor >= self.events.len()
    }

    /// 重置游标（新一局）。
    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    /// 取出所有 `at_secs <= time` 且尚未触发的 `tag`。
    pub fn drain_due(&mut self, time: f32) -> impl Iterator<Item = u32> + '_ {
        let start = self.cursor;
        while self.cursor < self.events.len() && self.events[self.cursor].at_secs <= time {
            self.cursor += 1;
        }
        self.events[start..self.cursor].iter().map(|e| e.tag)
    }
}
