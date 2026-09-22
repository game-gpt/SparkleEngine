//! 音频框架：输出设备 + 程序化短音 + Symphonia 文件解码（经 `spark-media`）。
//!
//! **不**包含曲库或玩法语义；游戏自行映射事件到 [`PlayRequest`] / `Tone` / 资源路径。
//!
//! 宿主侧基于 rodio 0.22：`MixerDeviceSink` + [`Player`]。

#![warn(missing_docs)]
use std::{
    f32::consts::PI,
    num::{NonZeroU16, NonZeroU32},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, buffer::SamplesBuffer, source::Source};
use spark_core::SparkError;

/// 重新导出：游戏可预解码后缓存。
pub use spark_media::{AudioDecoder, PcmAudio};

fn nz_u16(v: u16) -> NonZeroU16 {
    NonZeroU16::new(v.max(1)).expect("channels >= 1")
}

fn nz_u32(v: u32) -> NonZeroU32 {
    NonZeroU32::new(v.max(1)).expect("sample_rate >= 1")
}

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
    device: Option<MixerDeviceSink>,
    muted: bool,
}

impl AudioBus {
    /// 无输出设备的静默总线（测试 / 无音频宿主）。
    pub fn silent() -> Self {
        Self { device: None, muted: true }
    }

    pub fn try_open() -> Self {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(mut device) => {
                device.log_on_drop(false);
                tracing::info!(event = "spark.audio.output_ready");
                Self { device: Some(device), muted: false }
            }
            Err(err) => {
                tracing::warn!(event = "spark.audio.output_unavailable", ?err);
                Self { device: None, muted: true }
            }
        }
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    pub fn is_muted(&self) -> bool {
        self.muted || self.device.is_none()
    }

    /// 播放程序化正弦短音（非阻塞）。
    pub fn play_tone(&self, tone: Tone) {
        if self.is_muted() {
            return;
        }
        let Some(device) = self.device.as_ref()
        else {
            return;
        };
        let player = Player::connect_new(device.mixer());
        let vol = tone.volume.clamp(0.0, 1.0);
        player.set_volume(vol);
        player.append(SineWave::new(tone.freq_hz, tone.duration_ms));
        // 不变式：detach 后 rodio 在后台线程持有样本直到结束
        player.detach();
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
        let Some(device) = self.device.as_ref()
        else {
            return Ok(());
        };
        if pcm.samples.is_empty() {
            return Ok(());
        }
        let player = Player::connect_new(device.mixer());
        let buf = SamplesBuffer::new(nz_u16(pcm.channels), nz_u32(pcm.sample_rate), pcm.samples.clone());
        player.append(buf);
        player.detach();
        Ok(())
    }

    /// 播放 PCM 并返回可停止句柄。静音或无设备时返回 `Ok(None)`。
    ///
    /// `volume` 为 0..=1，`speed` 为 1.0 原速。`looping` 为真时循环到 [`Playback`] 停止。
    pub fn start_pcm(&self, pcm: &PcmAudio, volume: f32, speed: f32, looping: bool) -> Result<Option<Playback>, SparkError> {
        if self.is_muted() || pcm.samples.is_empty() {
            return Ok(None);
        }
        let Some(device) = self.device.as_ref()
        else {
            return Ok(None);
        };
        let player = Player::connect_new(device.mixer());
        player.set_volume(volume.clamp(0.0, 1.0));
        player.set_speed(speed.clamp(0.05, 4.0));
        player.append(PcmStream::new(pcm, looping));
        Ok(Some(Playback { player }))
    }
}

/// 持有中的播放。丢弃或 [`Playback::stop`] 时停止。
pub struct Playback {
    player: Player,
}

impl Playback {
    /// 停止并清空。
    pub fn stop(self) {
        self.player.stop();
    }

    /// 调整音量（0..=1）。
    pub fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume.clamp(0.0, 1.0));
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.player.stop();
    }
}

/// 交错 PCM 流。循环时在末尾回到 0。
pub struct PcmStream {
    samples: Arc<[f32]>,
    channels: NonZeroU16,
    sample_rate: NonZeroU32,
    index: usize,
    looping: bool,
}

impl PcmStream {
    /// 从 [`PcmAudio`] 构造 PCM 迭代器。`looping` 为真时在末尾回到起点。
    pub fn new(pcm: &PcmAudio, looping: bool) -> Self {
        Self {
            samples: Arc::from(pcm.samples.as_slice()),
            channels: nz_u16(pcm.channels),
            sample_rate: nz_u32(pcm.sample_rate),
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
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZeroU16 {
        self.channels
    }

    fn sample_rate(&self) -> NonZeroU32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        if self.looping {
            return None;
        }
        let frames = self.samples.len() / self.channels.get() as usize;
        Some(Duration::from_secs_f64(frames as f64 / self.sample_rate.get() as f64))
    }
}

/// 有限长正弦源。
struct SineWave {
    freq: f32,
    sample_rate: NonZeroU32,
    samples_left: usize,
    t: f32,
}

impl SineWave {
    fn new(freq: f32, duration_ms: u32) -> Self {
        let sample_rate = nz_u32(44_100);
        let samples = (sample_rate.get() as u64 * duration_ms as u64 / 1000) as usize;
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
        self.t += 1.0 / self.sample_rate.get() as f32;
        let fade = if self.samples_left < 256 { self.samples_left as f32 / 256.0 } else { 1.0 };
        Some(sample * fade)
    }
}

impl Source for SineWave {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples_left)
    }

    fn channels(&self) -> NonZeroU16 {
        nz_u16(1)
    }

    fn sample_rate(&self) -> NonZeroU32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let ms = self.samples_left as u64 * 1000 / self.sample_rate.get() as u64;
        Some(Duration::from_millis(ms))
    }
}
