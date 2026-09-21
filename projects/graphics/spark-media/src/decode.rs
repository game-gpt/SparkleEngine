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
    pub fn decode_path(path: impl AsRef<Path>, track_id: Option<u32>) -> Result<PcmAudio, MediaError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| MediaError::open_path(path, &e))?;
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        Self::decode_mss(MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default()), hint, track_id)
    }

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
