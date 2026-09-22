//! 自 `src/decode.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_media::*;

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
    let info = spark_media::probe_bytes(&tiny_wav()).unwrap();
    assert!(info.default_audio().is_some());
}
