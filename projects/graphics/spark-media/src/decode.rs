//! 音频轨 → 交错 f32 PCM。

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

use crate::MediaError;

/// 解码后的 PCM（交错，范围约 -1..=1）。
#[derive(Debug, Clone)]
pub struct PcmAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl PcmAudio {
    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            return 0;
        }
        self.samples.len() / self.channels as usize
    }

    pub fn duration_secs(&self) -> f64 {
        self.frame_count() as f64 / self.sample_rate.max(1) as f64
    }
}

/// 基于 Symphonia 的音频解码器。
pub struct AudioDecoder;

impl AudioDecoder {
    /// 解码文件中默认（或指定）音频轨为整段 PCM。
    pub fn decode_path(
        path: impl AsRef<Path>,
        track_id: Option<u32>,
    ) -> Result<PcmAudio, MediaError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| MediaError::open_path(path, &e))?;
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        Self::decode_mss(
            MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default()),
            hint,
            track_id,
        )
    }

    pub fn decode_bytes(
        bytes: impl Into<Vec<u8>>,
        track_id: Option<u32>,
    ) -> Result<PcmAudio, MediaError> {
        let cursor = std::io::Cursor::new(bytes.into());
        Self::decode_mss(
            MediaSourceStream::new(Box::new(cursor), MediaSourceStreamOptions::default()),
            Hint::new(),
            track_id,
        )
    }

    fn decode_mss(
        mss: MediaSourceStream,
        hint: Hint,
        track_id: Option<u32>,
    ) -> Result<PcmAudio, MediaError> {
        let probed = get_probe().format(
            &hint,
            mss,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )?;
        let mut format = probed.format;

        let track = if let Some(id) = track_id {
            format
                .tracks()
                .iter()
                .find(|t| t.id == id)
                .ok_or(MediaError::NoAudioTrack)?
                .clone()
        } else {
            format
                .tracks()
                .iter()
                .find(|t| {
                    t.codec_params.codec != CODEC_TYPE_NULL
                        && t.codec_params.sample_rate.is_some()
                })
                .cloned()
                .ok_or(MediaError::NoAudioTrack)?
        };

        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or(MediaError::NoAudioTrack)?;
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(1)
            .max(1);

        let mut decoder = get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

        let mut samples = Vec::new();
        let track_id = track.id;

        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(SymError::ResetRequired) => continue,
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };
            if packet.track_id() != track_id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(buf) => append_f32(&mut samples, &buf),
                Err(SymError::DecodeError(_)) => continue,
                Err(e) => return Err(e.into()),
            }
        }

        Ok(PcmAudio {
            sample_rate,
            channels,
            samples,
        })
    }
}

fn append_f32(out: &mut Vec<f32>, buf: &AudioBufferRef<'_>) {
    fn push_planes<T: Copy>(out: &mut Vec<f32>, planes: &[&[T]], map: impl Fn(T) -> f32) {
        let frames = planes.first().map(|p| p.len()).unwrap_or(0);
        out.reserve(frames * planes.len());
        for i in 0..frames {
            for plane in planes {
                out.push(map(plane[i]));
            }
        }
    }

    match buf {
        AudioBufferRef::F32(b) => push_planes(out, b.planes().planes(), |v| v),
        AudioBufferRef::U8(b) => push_planes(out, b.planes().planes(), |v| v as f32 / 128.0 - 1.0),
        AudioBufferRef::U16(b) => {
            push_planes(out, b.planes().planes(), |v| v as f32 / 32768.0 - 1.0)
        }
        AudioBufferRef::U24(b) => push_planes(out, b.planes().planes(), |v| {
            v.inner() as f32 / 8_388_608.0 - 1.0
        }),
        AudioBufferRef::U32(b) => {
            push_planes(out, b.planes().planes(), |v| v as f32 / 2_147_483_648.0 - 1.0)
        }
        AudioBufferRef::S8(b) => push_planes(out, b.planes().planes(), |v| v as f32 / 128.0),
        AudioBufferRef::S16(b) => push_planes(out, b.planes().planes(), |v| v as f32 / 32768.0),
        AudioBufferRef::S24(b) => {
            push_planes(out, b.planes().planes(), |v| v.inner() as f32 / 8_388_608.0)
        }
        AudioBufferRef::S32(b) => {
            push_planes(out, b.planes().planes(), |v| v as f32 / 2_147_483_648.0)
        }
        AudioBufferRef::F64(b) => push_planes(out, b.planes().planes(), |v| v as f32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 最小合法 PCM WAV（单声道 8bit 44100Hz，若干样本）。
    fn tiny_wav() -> Vec<u8> {
        let samples: Vec<u8> = (0..64).map(|i| 128u8.wrapping_add(i)).collect();
        let data_len = samples.len() as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&44100u32.to_le_bytes());
        v.extend_from_slice(&(44100u32).to_le_bytes()); // byte rate
        v.extend_from_slice(&1u16.to_le_bytes()); // block align
        v.extend_from_slice(&8u16.to_le_bytes()); // bits
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        v.extend_from_slice(&samples);
        v
    }

    #[test]
    fn decode_wav_bytes() {
        let pcm = AudioDecoder::decode_bytes(tiny_wav(), None).unwrap();
        assert_eq!(pcm.sample_rate, 44100);
        assert_eq!(pcm.channels, 1);
        assert!(!pcm.samples.is_empty());
    }

    #[test]
    fn probe_wav_audio_track() {
        let info = crate::probe_bytes(&tiny_wav()).unwrap();
        assert!(info.default_audio().is_some());
    }
}
