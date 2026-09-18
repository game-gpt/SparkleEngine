//! Symphonia 媒体基础设施：探测、解复用、音频 PCM 解码。
//!
//! - [`spark_audio`](../spark_audio) 用本 crate 解码后交给输出设备。
//! - [`spark_video`](../spark_video) 用本 crate 拉视频包与轨元数据（像素解码另议）。
//!
//! **无**曲库 / 过场脚本 / 游戏资源 ID 语义。

mod decode;
mod demux;
mod probe;

pub use decode::{AudioDecoder, PcmAudio};
pub use demux::{MediaPacket, MediaReader, PacketKind};
pub use probe::{probe_bytes, probe_path, AudioTrackInfo, MediaInfo, TrackInfo, VideoTrackInfo};

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use spark_core::{ErrorArg, SparkError};

/// 媒体层结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum MediaError {
    Open {
        path: Option<PathBuf>,
        detail: String,
    },
    NoAudioTrack,
    NoVideoTrack,
    UnsupportedCodec { codec: String },
    EndOfStream,
    Decode { detail: String },
}

impl MediaError {
    pub fn open(path: impl Into<Option<PathBuf>>, detail: impl Into<String>) -> Self {
        Self::Open {
            path: path.into(),
            detail: detail.into(),
        }
    }

    pub fn open_path(path: impl Into<PathBuf>, detail: impl Into<String>) -> Self {
        Self::Open {
            path: Some(path.into()),
            detail: detail.into(),
        }
    }

    pub fn decode(detail: impl Into<String>) -> Self {
        Self::Decode {
            detail: detail.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Open { .. } => "spark.media.open",
            Self::NoAudioTrack => "spark.media.no_audio_track",
            Self::NoVideoTrack => "spark.media.no_video_track",
            Self::UnsupportedCodec { .. } => "spark.media.unsupported_codec",
            Self::EndOfStream => "spark.media.end_of_stream",
            Self::Decode { .. } => "spark.media.decode",
        }
    }
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for MediaError {}

impl From<MediaError> for SparkError {
    fn from(e: MediaError) -> Self {
        let mut err = SparkError::new(spark_core::ErrorCode::parse(e.code()));
        match &e {
            MediaError::Open { path, detail } => {
                if let Some(p) = path {
                    err = err.arg(
                        "path",
                        ErrorArg::String(Arc::from(p.to_string_lossy().as_ref())),
                    );
                }
                err.arg("detail", ErrorArg::String(Arc::from(detail.as_str())))
            }
            MediaError::UnsupportedCodec { codec } => {
                err.arg("codec", ErrorArg::String(Arc::from(codec.as_str())))
            }
            MediaError::Decode { detail } => {
                err.arg("detail", ErrorArg::String(Arc::from(detail.as_str())))
            }
            _ => err,
        }
    }
}

impl From<symphonia::core::errors::Error> for MediaError {
    fn from(e: symphonia::core::errors::Error) -> Self {
        MediaError::decode(e.to_string())
    }
}
