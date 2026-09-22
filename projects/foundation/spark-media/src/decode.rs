//! 音频轨 → 交错 f32 PCM（Symphonia 0.6）。

use std::{fs::File, path::Path};

use symphonia::{
    core::{
        codecs::audio::AudioDecoderOptions,
        errors::Error as SymError,
        formats::{FormatOptions, FormatReader, probe::Hint},
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::MetadataOptions,
    },
    default::{get_codecs, get_probe},
};

use crate::MediaError;

/// 解码后的 PCM（交错，范围约 -1..=1）。
///
/// `samples` 布局为帧主序交错：`[L0, R0, L1, R1, ...]`（声道数见 `channels`）。
#[derive(Debug, Clone)]
pub struct PcmAudio {
    /// 采样率，单位 Hz。
    pub sample_rate: u32,
    /// 声道数；至少为 `1`。
    pub channels: u16,
    /// 交错 f32 样本；幅度约定约在 `-1.0..=1.0`。
    pub samples: Vec<f32>,
}

impl PcmAudio {
    /// 帧数 = `samples.len() / channels`；`channels == 0` 时返回 `0`。
    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            return 0;
        }
        self.samples.len() / self.channels as usize
    }

    /// 按时长估算的秒数（`frame_count / max(sample_rate, 1)`）。
    pub fn duration_secs(&self) -> f64 {
        self.frame_count() as f64 / self.sample_rate.max(1) as f64
    }
}

/// 基于 Symphonia 的音频解码器。
///
/// 无状态命名空间：全部入口为关联函数，一次调用读完整段 PCM。
pub struct AudioDecoder;

impl AudioDecoder {
    /// 解码文件中默认（或指定）音频轨为整段 PCM。
    ///
    /// `track_id` 为 `None` 时取默认音频轨；指定 ID 必须对应音频轨。
    ///
    /// # Errors
    ///
    /// - 打开失败 → [`MediaError::Open`]
    /// - 无音频轨 / 非音频 → [`MediaError::NoAudioTrack`]
    /// - 解码失败 → [`MediaError::Decode`]（单包 `DecodeError` 会跳过继续）
    pub fn decode_path(path: impl AsRef<Path>, track_id: Option<u32>) -> Result<PcmAudio, MediaError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| MediaError::open_path(path, &e))?;
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        Self::decode_mss(MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default()), hint, track_id)
    }

    /// 从内存字节解码默认（或指定）音频轨为整段 PCM。
    ///
    /// 轨选择与错误语义同 [`Self::decode_path`]。
    pub fn decode_bytes(bytes: impl Into<Vec<u8>>, track_id: Option<u32>) -> Result<PcmAudio, MediaError> {
        let cursor = std::io::Cursor::new(bytes.into());
        Self::decode_mss(MediaSourceStream::new(Box::new(cursor), MediaSourceStreamOptions::default()), Hint::new(), track_id)
    }

    fn decode_mss(mss: MediaSourceStream<'static>, hint: Hint, track_id: Option<u32>) -> Result<PcmAudio, MediaError> {
        let mut format: Box<dyn FormatReader> = get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())?;

        let track = if let Some(id) = track_id {
            format.tracks().iter().find(|t| t.id == id).cloned().ok_or(MediaError::NoAudioTrack)?
        }
        else {
            format
                .default_track(symphonia::core::formats::TrackType::Audio)
                .cloned()
                .or_else(|| format.tracks().iter().find(|t| t.codec_params.as_ref().is_some_and(|p| p.is_audio())).cloned())
                .ok_or(MediaError::NoAudioTrack)?
        };

        let audio_params = track.codec_params.as_ref().and_then(|p| p.audio()).cloned().ok_or(MediaError::NoAudioTrack)?;

        let sample_rate = audio_params.sample_rate.ok_or(MediaError::NoAudioTrack)?;
        let channels = audio_params.channels.as_ref().map(|c| c.count() as u16).unwrap_or(1).max(1);

        let mut decoder = get_codecs().make_audio_decoder(&audio_params, &AudioDecoderOptions::default())?;

        let mut samples = Vec::new();
        let track_id = track.id;

        loop {
            let packet = match format.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => break,
                Err(SymError::ResetRequired) => continue,
                Err(e) => return Err(e.into()),
            };
            if packet.track_id != track_id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(buf) => buf.copy_to_vec_interleaved(&mut samples),
                Err(SymError::DecodeError(_)) => continue,
                Err(e) => return Err(e.into()),
            }
        }

        Ok(PcmAudio { sample_rate, channels, samples })
    }
}
