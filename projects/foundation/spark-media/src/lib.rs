//! Symphonia 媒体基础设施：探测、解复用、音频 PCM 解码。
//!
//! - [`spark_audio`](../spark_audio) 用本 crate 解码后交给输出设备。
//! - [`spark_video`](../spark_video) 用本 crate 拉视频包与轨元数据（像素解码另议）。
//!
//! **无**曲库 / 过场脚本 / 游戏资源 ID 语义。

#![warn(missing_docs)]
mod decode;
mod demux;
mod probe;

pub use decode::{AudioDecoder, PcmAudio};
pub use demux::{MediaPacket, MediaReader, PacketKind};
pub use probe::{AudioTrackInfo, MediaInfo, TrackInfo, VideoTrackInfo, probe_bytes, probe_path};

use std::{fmt, io, path::PathBuf, sync::Arc};

use spark_core::{ErrorArg, ErrorArgs, SparkError};

/// 媒体层结构化错误。`Display` 只输出稳定码。
#[derive(Debug)]
pub enum MediaError {
    /// 打开失败：路径事实 + IO kind 令牌（非 OS 本地化句子）。
    Open {
        path: Option<PathBuf>,
        io_kind: &'static str,
    },
    NoAudioTrack,
    NoVideoTrack,
    UnsupportedCodec {
        codec: String,
    },
    EndOfStream,
    /// 解码失败：Symphonia / 容器层稳定 kind 令牌。
    Decode {
        kind: &'static str,
    },
}

impl MediaError {
    pub fn open_io(path: impl Into<Option<PathBuf>>, kind: io::ErrorKind) -> Self {
        Self::Open { path: path.into(), io_kind: io_kind_token(kind) }
    }

    pub fn open_path(path: impl Into<PathBuf>, err: &io::Error) -> Self {
        Self::Open { path: Some(path.into()), io_kind: io_kind_token(err.kind()) }
    }

    pub fn decode_kind(kind: &'static str) -> Self {
        Self::Decode { kind }
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

    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Open { path, io_kind } => {
                let mut args = ErrorArgs::new().with("io_kind", ErrorArg::String(Arc::from(*io_kind)));
                if let Some(p) = path {
                    args.insert("path", ErrorArg::Path(Arc::from(p.to_string_lossy().as_ref())));
                }
                args
            }
            Self::UnsupportedCodec { codec } => ErrorArgs::new().with("codec", ErrorArg::String(Arc::from(codec.as_str()))),
            Self::Decode { kind } => ErrorArgs::new().with("kind", ErrorArg::String(Arc::from(*kind))),
            _ => ErrorArgs::new(),
        }
    }
}

fn io_kind_token(kind: io::ErrorKind) -> &'static str {
    use io::ErrorKind::*;
    match kind {
        NotFound => "not_found",
        PermissionDenied => "permission_denied",
        InvalidData => "invalid_data",
        UnexpectedEof => "unexpected_eof",
        AlreadyExists => "already_exists",
        TimedOut => "timed_out",
        _ => "other",
    }
}

fn symphonia_kind(e: &symphonia::core::errors::Error) -> &'static str {
    use symphonia::core::errors::Error::*;
    match e {
        IoError(_) => "io",
        DecodeError(_) => "decode",
        SeekError(_) => "seek",
        Unsupported(_) => "unsupported",
        LimitError(_) => "limit",
        ResetRequired => "reset_required",
        // Symphonia 将 Error 标为 non_exhaustive。
        _ => "other",
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
        SparkError::new(spark_core::ErrorCode::parse(e.code())).with_args(e.args())
    }
}

impl From<symphonia::core::errors::Error> for MediaError {
    fn from(e: symphonia::core::errors::Error) -> Self {
        MediaError::decode_kind(symphonia_kind(&e))
    }
}
