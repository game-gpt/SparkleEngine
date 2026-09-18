//! 容器探测与轨元数据。
//!
//! Symphonia 0.5 以音频编解码为主；无 `sample_rate` 的轨视为视频/其它轨候选，
//! 宽高在容器未暴露时为 `0`（后续可由专用解码器补全）。

use std::fs::File;
use std::path::Path;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

use crate::MediaError;

/// 音频轨摘要。
#[derive(Debug, Clone)]
pub struct AudioTrackInfo {
    pub track_id: u32,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
}

/// 视频轨摘要（解复用侧；像素解码由 `spark-video` / 渲染路径另接）。
#[derive(Debug, Clone)]
pub struct VideoTrackInfo {
    pub track_id: u32,
    pub codec: String,
    /// 像素宽；容器未提供时为 0。
    pub width: u32,
    /// 像素高；容器未提供时为 0。
    pub height: u32,
    pub time_base_numer: u32,
    pub time_base_denom: u32,
}

#[derive(Debug, Clone)]
pub enum TrackInfo {
    Audio(AudioTrackInfo),
    Video(VideoTrackInfo),
    Other { track_id: u32, codec: String },
}

#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub tracks: Vec<TrackInfo>,
    pub format: String,
}

impl MediaInfo {
    pub fn audio_tracks(&self) -> impl Iterator<Item = &AudioTrackInfo> {
        self.tracks.iter().filter_map(|t| match t {
            TrackInfo::Audio(a) => Some(a),
            _ => None,
        })
    }

    pub fn video_tracks(&self) -> impl Iterator<Item = &VideoTrackInfo> {
        self.tracks.iter().filter_map(|t| match t {
            TrackInfo::Video(v) => Some(v),
            _ => None,
        })
    }

    pub fn default_audio(&self) -> Option<&AudioTrackInfo> {
        self.audio_tracks().next()
    }

    pub fn default_video(&self) -> Option<&VideoTrackInfo> {
        self.video_tracks().next()
    }
}

pub fn probe_path(path: impl AsRef<Path>) -> Result<MediaInfo, MediaError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|e| MediaError::open_path(path, &e))?;
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    probe_mss(
        MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default()),
        hint,
    )
}

pub fn probe_bytes(bytes: &[u8]) -> Result<MediaInfo, MediaError> {
    let cursor = std::io::Cursor::new(bytes.to_vec());
    probe_mss(
        MediaSourceStream::new(Box::new(cursor), MediaSourceStreamOptions::default()),
        Hint::new(),
    )
}

pub(crate) fn classify_tracks<'a>(
    tracks: impl IntoIterator<Item = &'a symphonia::core::formats::Track>,
) -> MediaInfo {
    let mut out = Vec::new();
    for track in tracks {
        let id = track.id;
        let codec = format!("{:?}", track.codec_params.codec);
        if let Some(rate) = track.codec_params.sample_rate {
            let ch = track
                .codec_params
                .channels
                .map(|c| c.count() as u16)
                .unwrap_or(1);
            out.push(TrackInfo::Audio(AudioTrackInfo {
                track_id: id,
                codec,
                sample_rate: rate,
                channels: ch.max(1),
            }));
        } else if track.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL {
            // 无采样率的已注册编解码轨 → 视频候选（Symphonia 不提供宽高字段）
            let (numer, denom) = track
                .codec_params
                .time_base
                .map(|tb| (tb.numer, tb.denom))
                .unwrap_or((1, 1));
            out.push(TrackInfo::Video(VideoTrackInfo {
                track_id: id,
                codec,
                width: 0,
                height: 0,
                time_base_numer: numer,
                time_base_denom: denom.max(1),
            }));
        } else {
            out.push(TrackInfo::Other {
                track_id: id,
                codec,
            });
        }
    }
    MediaInfo {
        tracks: out,
        format: "symphonia".into(),
    }
}

fn probe_mss(mss: MediaSourceStream, hint: Hint) -> Result<MediaInfo, MediaError> {
    let probed = get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    Ok(classify_tracks(probed.format.tracks()))
}
