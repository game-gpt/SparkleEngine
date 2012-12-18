//! 音频框架：输出设备 + 程序化短音 + Symphonia 文件解码（经 `spark-media`）。
//!
//! **不**包含曲库或玩法语义；游戏自行映射事件到 [`PlayRequest`] / `Tone` / 资源路径。

use std::{
    f32::consts::PI,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use rodio::{OutputStream, OutputStreamHandle, Sink, buffer::SamplesBuffer, source::Source};
use spark_core::{SparkError, codes};

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
        Self { freq_hz, duration_ms, volume }
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
                        }
                        else {
                            tracing::warn!(
                                event = "spark.audio.play_file_failed",
                                error = %e,
                                path = %path.display()
                            );
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
                tracing::info!(event = "spark.audio.output_ready");
                Self { _stream: Some(stream), handle: Some(handle), muted: false }
            }
            Err(err) => {
                tracing::warn!(event = "spark.audio.output_unavailable", ?err);
                Self { _stream: None, handle: None, muted: true }
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
        let Some(handle) = self.handle.as_ref()
        else {
            return;
        };
        let Ok(sink) = Sink::try_new(handle)
        else {
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
        let Some(handle) = self.handle.as_ref()
        else {
            return Ok(());
        };
        let sink = Sink::try_new(handle).map_err(|e| SparkError::new(codes::audio_sink()).caused_by(e))?;
        if pcm.samples.is_empty() {
            return Ok(());
        }
        let buf = SamplesBuffer::new(pcm.channels, pcm.sample_rate, pcm.samples.clone());
        sink.append(buf);
        sink.detach();
        Ok(())
    }

    /// 播放 PCM 并返回可停止句柄。静音或无设备时返回 `Ok(None)`。
    ///
    /// `volume` 为 0..=1，`speed` 为 1.0 原速。`looping` 为真时循环到 [`Playback`] 停止。
    pub fn start_pcm(&self, pcm: &PcmAudio, volume: f32, speed: f32, looping: bool) -> Result<Option<Playback>, SparkError> {
        if self.is_muted() || pcm.samples.is_empty() {
            return Ok(None);
        }
        let Some(handle) = self.handle.as_ref()
        else {
            return Ok(None);
        };
        let sink = Sink::try_new(handle).map_err(|e| SparkError::new(codes::audio_sink()).caused_by(e))?;
        sink.set_volume(volume.clamp(0.0, 1.0));
        sink.set_speed(speed.clamp(0.05, 4.0));
        sink.append(PcmStream::new(pcm, looping));
        Ok(Some(Playback { sink }))
    }
}

/// 持有中的播放。丢弃或 [`Playback::stop`] 时停止。
pub struct Playback {
    sink: Sink,
}

impl Playback {
    /// 停止并清空。
    pub fn stop(self) {
        self.sink.stop();
    }

    /// 调整音量（0..=1）。
    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume.clamp(0.0, 1.0));
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.sink.stop();
    }
}

/// 交错 PCM 流。循环时在末尾回到 0。
struct PcmStream {
    samples: Arc<[f32]>,
    channels: u16,
    sample_rate: u32,
    index: usize,
    looping: bool,
}

impl PcmStream {
    fn new(pcm: &PcmAudio, looping: bool) -> Self {
        Self {
            samples: Arc::from(pcm.samples.as_slice()),
            channels: pcm.channels.max(1),
            sample_rate: pcm.sample_rate.max(1),
            index: 0,
            looping,
        }
    }
}

impl Iterator for PcmStream {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.samples.is_empty() {
            return None;
        }
        if self.index >= self.samples.len() {
            if !self.looping {
                return None;
            }
            self.index = 0;
        }
        let sample = self.samples[self.index];
        self.index += 1;
        Some(sample)
    }
}

impl Source for PcmStream {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        if self.looping {
            return None;
        }
        let frames = self.samples.len() / self.channels as usize;
        Some(Duration::from_secs_f64(frames as f64 / self.sample_rate as f64))
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
        Self { freq: freq.max(20.0), sample_rate, samples_left: samples.max(1), t: 0.0 }
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
        let fade = if self.samples_left < 256 { self.samples_left as f32 / 256.0 } else { 1.0 };
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
        let bus = AudioBus { _stream: None, handle: None, muted: true };
        let mut q = AudioQueue::new();
        q.play_tone(Tone::new(440.0, 10, 0.5));
        q.play_file("nope.wav");
        assert_eq!(q.len(), 2);
        let mut errs = Vec::new();
        q.flush(&bus, Some(&mut errs));
        assert!(q.is_empty());
        assert!(errs.is_empty());
    }

    #[test]
    fn pcm_stream_loops_then_stops() {
        let pcm = PcmAudio { sample_rate: 4, channels: 1, samples: vec![0.1, 0.2] };
        let mut looping = PcmStream::new(&pcm, true);
        assert_eq!(looping.next(), Some(0.1));
        assert_eq!(looping.next(), Some(0.2));
        assert_eq!(looping.next(), Some(0.1));
        let mut once = PcmStream::new(&pcm, false);
        assert_eq!(once.next(), Some(0.1));
        assert_eq!(once.next(), Some(0.2));
        assert_eq!(once.next(), None);
    }

    #[test]
    fn muted_start_pcm_returns_none() {
        let bus = AudioBus { _stream: None, handle: None, muted: true };
        let pcm = PcmAudio { sample_rate: 8, channels: 1, samples: vec![0.0, 1.0] };
        assert!(bus.start_pcm(&pcm, 1.0, 1.0, false).unwrap().is_none());
    }
}
