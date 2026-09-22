//! Symphonia 媒体基础设施：探测、解复用、音频 PCM 解码。
//!
//! - [`spark_audio`](../spark_audio) 用本 crate 解码后交给输出设备。
//! - [`spark_video`](../spark_video) 用本 crate 拉视频包与轨元数据（像素解码另议）。
//!
//! **无**曲库 / 过场脚本 / 游戏资源 ID 语义。

#![forbid(missing_docs)]
mod decode;
mod demux;
mod probe;

pub use decode::{AudioDecoder, PcmAudio};
pub use demux::{MediaPacket, MediaReader, PacketKind};
pub use probe::{AudioTrackInfo, MediaInfo, TrackInfo, VideoTrackInfo, probe_bytes, probe_path};

use std::{fmt, io, path::PathBuf, sync::Arc};

use spark_types::{ErrorArg, ErrorArgs, SparkError};

/// 媒体层结构化错误。`Display` 只输出稳定码。
///
/// 稳定码经 [`Self::code`] 暴露；结构化参数经 [`Self::args`] 进入 [`SparkError`]。
/// 不变式：`Display` / `code` 不含 OS 本地化句子或自然语言 detail。
#[derive(Debug)]
pub enum MediaError {
    /// 打开失败：路径事实 + IO kind 令牌（非 OS 本地化句子）。
    Open {
        /// 失败时的路径；内存字节源打开失败时为 `None`。
        path: Option<PathBuf>,
        /// `io::ErrorKind` 的稳定令牌（如 `not_found`），见内部 `io_kind_token`。
        io_kind: &'static str,
    },
    /// 容器中无可用音频轨，或指定轨不是音频。
    NoAudioTrack,
    /// 容器中无可用视频轨，或指定轨不是视频。
    NoVideoTrack,
    /// 编解码器不被当前 Symphonia 能力集支持。
    UnsupportedCodec {
        /// 编解码器标识字符串（来自 codec 参数的 `Debug` 形式）。
        codec: String,
    },
    /// 已到流末尾（部分上层将 EOS 当错误而非 `None` 时使用）。
    EndOfStream,
    /// 解码失败：Symphonia / 容器层稳定 kind 令牌。
    Decode {
        /// Symphonia 错误分类令牌（如 `io` / `decode` / `seek` / `unsupported`）。
        kind: &'static str,
    },
}

impl MediaError {
    /// 由可选路径与 `io::ErrorKind` 构造 [`Self::Open`]；`kind` 映射为稳定令牌。
    pub fn open_io(path: impl Into<Option<PathBuf>>, kind: io::ErrorKind) -> Self {
        Self::Open { path: path.into(), io_kind: io_kind_token(kind) }
    }

    /// 由路径与 `io::Error` 构造 [`Self::Open`]；保留路径事实与 kind 令牌。
    pub fn open_path(path: impl Into<PathBuf>, err: &io::Error) -> Self {
        Self::Open { path: Some(path.into()), io_kind: io_kind_token(err.kind()) }
    }

    /// 由稳定 kind 令牌构造 [`Self::Decode`]。
    pub fn decode_kind(kind: &'static str) -> Self {
        Self::Decode { kind }
    }

    /// 稳定错误码（`spark.media.*`）；与 `Display` 输出一致。
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

    /// 结构化参数：路径 / `io_kind` / `codec` / `kind` 等机器可读字段。
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
        SparkError::new(spark_types::ErrorCode::parse(e.code())).with_args(e.args())
    }
}

impl From<symphonia::core::errors::Error> for MediaError {
    fn from(e: symphonia::core::errors::Error) -> Self {
        MediaError::decode_kind(symphonia_kind(&e))
    }
}
