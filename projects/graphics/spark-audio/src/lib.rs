//! 音频框架：输出设备 + 程序化短音 + Symphonia 文件解码（经 `spark-media`）。
//!
//! **不**包含曲库或玩法语义；游戏自行映射事件到 [`PlayRequest`] / `Tone` / 资源路径。

use std::f32::consts::PI;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rodio::buffer::SamplesBuffer;
use rodio::source::Source;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use spark_core::SparkError;

/// 重新导出：游戏可预解码后缓存。
pub use spark_media::{AudioDecoder, PcmAudio};

/// 一段短音描述（Hz / 毫秒 / 音量 0..=1）。
#[derive(Debug, Clone, Copy)]
pub struct Tone {
    pub freq_hz: f32,
    pub duration_ms: u32,
    pub volume: f32,
}

impl Tone {
    pub const fn new(freq_hz: f32, duration_ms: u32, volume: f32) -> Self {
        Self {
            freq_hz,
            duration_ms,
            volume,
        }
    }
}

/// 播放请求（事件式，由宿主在帧末 `flush`）。
#[derive(Debug, Clone)]
pub enum PlayRequest {
    Tone(Tone),
    File(PathBuf),
}

/// 待播放队列。系统只 `push`，音频宿主 `flush` 到 [`AudioBus`]。
#[derive(Debug, Default)]
pub struct AudioQueue {
    pending: Vec<PlayRequest>,
}

impl AudioQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, req: PlayRequest) {
        self.pending.push(req);
    }

    pub fn play_tone(&mut self, tone: Tone) {
        self.push(PlayRequest::Tone(tone));
    }

    pub fn play_file(&mut self, path: impl Into<PathBuf>) {
        self.push(PlayRequest::File(path.into()));
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// 排空队列并交给总线播放。文件失败写入 `errors`（若提供）。
    pub fn flush(&mut self, bus: &AudioBus, mut errors: Option<&mut Vec<SparkError>>) {
        for req in self.pending.drain(..) {
            match req {
                PlayRequest::Tone(t) => bus.play_tone(t),
                PlayRequest::File(path) => {
                    if let Err(e) = bus.play_file(&path) {
                        if let Some(out) = errors.as_deref_mut() {
                            out.push(e);
                        } else {
                            tracing::warn!(error = %e, path = %path.display(), "音频文件播放失败");
                        }
                    }
                }
            }
        }
    }
}

/// 音频总线。创建设备失败时降级为静默（不崩游戏）。
pub struct AudioBus {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    muted: bool,
}

impl AudioBus {
    pub fn try_open() -> Self {
        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                tracing::info!("音频输出已就绪");
                Self {
                    _stream: Some(stream),
                    handle: Some(handle),
                    muted: false,
                }
            }
            Err(err) => {
                tracing::warn!(?err, "音频输出不可用，静默运行");
                Self {
                    _stream: None,
                    handle: None,
                    muted: true,
                }
            }
        }
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    pub fn is_muted(&self) -> bool {
        self.muted || self.handle.is_none()
    }

    /// 播放程序化正弦短音（非阻塞）。
    pub fn play_tone(&self, tone: Tone) {
        if self.is_muted() {
            return;
        }
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle) else {
            return;
        };
        let vol = tone.volume.clamp(0.0, 1.0);
        sink.set_volume(vol);
        let src = SineWave::new(tone.freq_hz, tone.duration_ms);
        sink.append(src);
        // 不变式：detach 后 rodio 在后台线程持有样本直到结束
        sink.detach();
    }

    /// 用 Symphonia（`spark-media`）解码文件并播放。
    pub fn play_file(&self, path: impl AsRef<Path>) -> Result<(), SparkError> {
        if self.is_muted() {
            return Ok(());
        }
        let pcm = AudioDecoder::decode_path(path, None)?;
        self.play_pcm(&pcm)
    }

    /// 播放已解码 PCM。
    pub fn play_pcm(&self, pcm: &PcmAudio) -> Result<(), SparkError> {
        if self.is_muted() {
            return Ok(());
        }
        let Some(handle) = self.handle.as_ref() else {
            return Ok(());
        };
        let sink = Sink::try_new(handle)
            .map_err(|e| SparkError::internal(format!("创建音频 Sink 失败：{e}")))?;
        if pcm.samples.is_empty() {
            return Ok(());
        }
        let buf = SamplesBuffer::new(pcm.channels, pcm.sample_rate, pcm.samples.clone());
        sink.append(buf);
        sink.detach();
        Ok(())
    }
}

/// 有限长正弦源。
struct SineWave {
    freq: f32,
    sample_rate: u32,
    samples_left: usize,
    t: f32,
}

impl SineWave {
    fn new(freq: f32, duration_ms: u32) -> Self {
        let sample_rate = 44_100;
        let samples = (sample_rate as u64 * duration_ms as u64 / 1000) as usize;
        Self {
            freq: freq.max(20.0),
            sample_rate,
            samples_left: samples.max(1),
            t: 0.0,
        }
    }
}

impl Iterator for SineWave {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.samples_left == 0 {
            return None;
        }
        self.samples_left -= 1;
        let sample = (self.t * self.freq * 2.0 * PI).sin() * 0.25;
        self.t += 1.0 / self.sample_rate as f32;
        let fade = if self.samples_left < 256 {
            self.samples_left as f32 / 256.0
        } else {
            1.0
        };
        Some(sample * fade)
    }
}

impl Source for SineWave {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples_left)
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let ms = self.samples_left as u64 * 1000 / self.sample_rate as u64;
        Some(Duration::from_millis(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_drains_into_silent_bus() {
        let bus = AudioBus {
            _stream: None,
            handle: None,
            muted: true,
        };
        let mut q = AudioQueue::new();
        q.play_tone(Tone::new(440.0, 10, 0.5));
        q.play_file("nope.wav");
        assert_eq!(q.len(), 2);
        let mut errs = Vec::new();
        q.flush(&bus, Some(&mut errs));
        assert!(q.is_empty());
        assert!(errs.is_empty());
    }
}
