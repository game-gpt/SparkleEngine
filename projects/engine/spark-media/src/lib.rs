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

use spark_core::SparkError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("{0}")]
    Message(String),
    #[error("无音频轨")]
    NoAudioTrack,
    #[error("无视频轨")]
    NoVideoTrack,
    #[error("编解码不支持：{0}")]
    UnsupportedCodec(String),
    #[error("已到流末尾")]
    EndOfStream,
}

impl From<MediaError> for SparkError {
    fn from(e: MediaError) -> Self {
        SparkError::Message(e.to_string())
    }
}

impl From<symphonia::core::errors::Error> for MediaError {
    fn from(e: symphonia::core::errors::Error) -> Self {
        MediaError::Message(e.to_string())
    }
}
