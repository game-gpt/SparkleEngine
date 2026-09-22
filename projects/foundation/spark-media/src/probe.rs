//! 容器探测与轨元数据（Symphonia 0.6）。
//!
//! 音频 / 视频 / 其它轨按 `CodecParameters` 枚举分类；视频宽高来自视频 codec 参数。

use std::{fs::File, path::Path};

use symphonia::{
    core::{
        codecs::CodecParameters,
        formats::{FormatOptions, FormatReader, probe::Hint},
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::MetadataOptions,
    },
    default::get_probe,
};

use crate::MediaError;

/// 音频轨摘要。
///
/// 字段来自容器 `CodecParameters::Audio`；缺省采样率记为 `0`，声道至少为 `1`。
#[derive(Debug, Clone)]
pub struct AudioTrackInfo {
    /// 容器内轨 ID（Symphonia `Track::id`）。
    pub track_id: u32,
    /// 编解码器标识（`Debug` 字符串，非稳定协议名）。
    pub codec: String,
    /// 采样率，单位 Hz；容器未声明时为 `0`。
    pub sample_rate: u32,
    /// 声道数；缺失时按 `1`，且不会小于 `1`。
    pub channels: u16,
}

/// 视频轨摘要（解复用侧；像素解码由 `spark-video` / 渲染路径另接）。
#[derive(Debug, Clone)]
pub struct VideoTrackInfo {
    /// 容器内轨 ID（Symphonia `Track::id`）。
    pub track_id: u32,
    /// 编解码器标识（`Debug` 字符串）。
    pub codec: String,
    /// 像素宽；容器未提供时为 0。
    pub width: u32,
    /// 像素高；容器未提供时为 0。
    pub height: u32,
    /// 轨 time_base 分子；缺省为 `1`。
    pub time_base_numer: u32,
    /// 轨 time_base 分母；缺省至少为 `1`（避免除零）。
    pub time_base_denom: u32,
}

/// 单条轨的分类摘要。
#[derive(Debug, Clone)]
pub enum TrackInfo {
    /// 音频轨。
    Audio(AudioTrackInfo),
    /// 视频轨。
    Video(VideoTrackInfo),
    /// 非音视频或参数缺失的轨。
    Other {
        /// 容器内轨 ID。
        track_id: u32,
        /// 编解码器或 `unknown`。
        codec: String,
    },
}

/// 探测结果：全部轨摘要 + 格式占位名。
///
/// `format` 当前固定为 `"symphonia"`，不承诺等于文件扩展名。
#[derive(Debug, Clone)]
pub struct MediaInfo {
    /// 按容器顺序排列的轨摘要。
    pub tracks: Vec<TrackInfo>,
    /// 探测后端占位名（当前为 `"symphonia"`）。
    pub format: String,
}

impl MediaInfo {
    /// 仅音频轨的迭代器（保持 `tracks` 原序）。
    pub fn audio_tracks(&self) -> impl Iterator<Item = &AudioTrackInfo> {
        self.tracks.iter().filter_map(|t| match t {
            TrackInfo::Audio(a) => Some(a),
            _ => None,
        })
    }

    /// 仅视频轨的迭代器（保持 `tracks` 原序）。
    pub fn video_tracks(&self) -> impl Iterator<Item = &VideoTrackInfo> {
        self.tracks.iter().filter_map(|t| match t {
            TrackInfo::Video(v) => Some(v),
            _ => None,
        })
    }

    /// 第一条音频轨；无音频时返回 `None`。
    pub fn default_audio(&self) -> Option<&AudioTrackInfo> {
        self.audio_tracks().next()
    }

    /// 第一条视频轨；无视频时返回 `None`。
    pub fn default_video(&self) -> Option<&VideoTrackInfo> {
        self.video_tracks().next()
    }
}

/// 从文件系统路径探测容器；扩展名写入 Symphonia `Hint`。
///
/// # Errors
///
/// 打开失败返回 [`MediaError::Open`]；探测/格式错误映射为 [`MediaError::Decode`]。
pub fn probe_path(path: impl AsRef<Path>) -> Result<MediaInfo, MediaError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|e| MediaError::open_path(path, &e))?;
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    probe_mss(MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default()), hint)
}

/// 从内存字节探测容器（无扩展名 hint）。
///
/// # Errors
///
/// 探测/格式错误映射为 [`MediaError::Decode`]。
pub fn probe_bytes(bytes: &[u8]) -> Result<MediaInfo, MediaError> {
    let cursor = std::io::Cursor::new(bytes.to_vec());
    probe_mss(MediaSourceStream::new(Box::new(cursor), MediaSourceStreamOptions::default()), Hint::new())
}

pub(crate) fn classify_tracks<'a>(tracks: impl IntoIterator<Item = &'a symphonia::core::formats::Track>) -> MediaInfo {
    let mut out = Vec::new();
    for track in tracks {
        let id = track.id;
        let (numer, denom) = track.time_base.map(|tb| (tb.numer.get(), tb.denom.get())).unwrap_or((1, 1));
        match track.codec_params.as_ref() {
            Some(CodecParameters::Audio(params)) => {
                let codec = format!("{:?}", params.codec);
                let rate = params.sample_rate.unwrap_or(0);
                let ch = params.channels.as_ref().map(|c| c.count() as u16).unwrap_or(1).max(1);
                out.push(TrackInfo::Audio(AudioTrackInfo { track_id: id, codec, sample_rate: rate, channels: ch }));
            }
            Some(CodecParameters::Video(params)) => {
                let codec = format!("{:?}", params.codec);
                out.push(TrackInfo::Video(VideoTrackInfo {
                    track_id: id,
                    codec,
                    width: params.width.map(u32::from).unwrap_or(0),
                    height: params.height.map(u32::from).unwrap_or(0),
                    time_base_numer: numer,
                    time_base_denom: denom.max(1),
                }));
            }
            Some(other) => {
                out.push(TrackInfo::Other { track_id: id, codec: format!("{other:?}") });
            }
            None => {
                out.push(TrackInfo::Other { track_id: id, codec: "unknown".into() });
            }
        }
    }
    MediaInfo { tracks: out, format: "symphonia".into() }
}

fn probe_mss(mss: MediaSourceStream<'static>, hint: Hint) -> Result<MediaInfo, MediaError> {
    let format: Box<dyn FormatReader> = get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())?;
    Ok(classify_tracks(format.tracks()))
}
