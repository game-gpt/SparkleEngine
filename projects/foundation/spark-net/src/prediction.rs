//! 客户端预测时钟。

use std::fmt;

use spark_types::ErrorArgs;

use crate::channel::Sequence;

/// 预测时钟错误。`Display` 只输出稳定码。
#[derive(Debug, PartialEq, Eq)]
pub enum PredictionError {
    /// [`PredictionClock::acknowledge`] 收到比 `last_acked_tick` 更早的 tick（时间回退）。
    TickRewind,
    /// [`PredictionClock::push_input`] 时待确认缓冲已达容量上限。
    BufferOverflow,
}

impl PredictionError {
    /// 稳定错误码（如 `spark.net.prediction.tick_rewind`）。
    pub fn code(&self) -> &'static str {
        match self {
            Self::TickRewind => "spark.net.prediction.tick_rewind",
            Self::BufferOverflow => "spark.net.prediction.buffer_overflow",
        }
    }

    /// 当前变体无额外结构化参数；返回空 [`ErrorArgs`]。
    pub fn args(&self) -> ErrorArgs {
        ErrorArgs::new()
    }
}

impl fmt::Display for PredictionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for PredictionError {}

/// 本地预测 tick 与待确认输入序号。
///
/// 不变式：`pending` 中 tick 单调递增；`acknowledge(tick)` 丢弃 `<= tick` 的项。
#[derive(Debug, Clone)]
pub struct PredictionClock {
    /// 本地已推进的仿真 tick（每次 [`Self::push_input`] 递增，环绕 `u32`）。
    pub local_tick: u32,
    /// 权威已确认到的最高 tick（含）；更早的预测输入已丢弃。
    pub last_acked_tick: u32,
    /// 下一条本地输入将使用的序号。
    pub next_input_seq: Sequence,
    pending: Vec<(u32, Sequence)>,
    cap: usize,
}

impl PredictionClock {
    /// 创建时钟；`buffer_cap` 为待确认输入条数上限，至少为 1。
    pub fn new(buffer_cap: usize) -> Self {
        Self { local_tick: 0, last_acked_tick: 0, next_input_seq: Sequence(0), pending: Vec::new(), cap: buffer_cap.max(1) }
    }

    /// 推进本地 tick 并登记一条输入序号。
    ///
    /// 成功时返回本条输入的 [`Sequence`]；缓冲已满则 [`PredictionError::BufferOverflow`]。
    pub fn push_input(&mut self) -> Result<Sequence, PredictionError> {
        if self.pending.len() >= self.cap {
            return Err(PredictionError::BufferOverflow);
        }
        let seq = self.next_input_seq;
        self.next_input_seq = seq.next();
        self.local_tick = self.local_tick.wrapping_add(1);
        self.pending.push((self.local_tick, seq));
        Ok(seq)
    }

    /// 权威确认到 `tick`（含），丢弃更早的预测输入。
    ///
    /// 成功时返回被丢弃的条数；`tick < last_acked_tick` 则 [`PredictionError::TickRewind`]。
    pub fn acknowledge(&mut self, tick: u32) -> Result<usize, PredictionError> {
        if tick < self.last_acked_tick {
            return Err(PredictionError::TickRewind);
        }
        self.last_acked_tick = tick;
        let before = self.pending.len();
        self.pending.retain(|(t, _)| *t > tick);
        Ok(before - self.pending.len())
    }

    /// 当前尚未被权威确认的输入条数。
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}
