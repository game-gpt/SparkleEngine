//! 视频框架：基于 `spark-media`（Symphonia）解复用视频轨。
//!
//! 当前交付轨元数据与压缩包泵；像素帧解码留给渲染/硬件路径。
//! **无**过场剧本或游戏镜头语义。

use std::path::Path;
use std::time::Duration;

use spark_core::SparkError;
use spark_media::{
    MediaPacket, MediaReader, PacketKind, VideoTrackInfo,
};

/// 打开中的视频剪辑（可含同文件音频轨 ID，供上层 AV 同步）。
pub struct VideoClip {
    reader: MediaReader,
    video: VideoTrackInfo,
    audio_track_id: Option<u32>,
}

impl VideoClip {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SparkError> {
        let reader = MediaReader::open_path(path).map_err(SparkError::from)?;
        let video = reader
            .default_video()
            .cloned()
            .ok_or_else(|| SparkError::internal("媒体中无视频轨"))?;
        let audio_track_id = reader.default_audio().map(|a| a.track_id);
        Ok(Self {
            reader,
            video,
            audio_track_id,
        })
    }

    pub fn open_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, SparkError> {
        let reader = MediaReader::open_bytes(bytes).map_err(SparkError::from)?;
        let video = reader
            .default_video()
            .cloned()
            .ok_or_else(|| SparkError::internal("媒体中无视频轨"))?;
        let audio_track_id = reader.default_audio().map(|a| a.track_id);
        Ok(Self {
            reader,
            video,
            audio_track_id,
        })
    }

    pub fn video_info(&self) -> &VideoTrackInfo {
        &self.video
    }

    pub fn audio_track_id(&self) -> Option<u32> {
        self.audio_track_id
    }

    pub fn width(&self) -> u32 {
        self.video.width
    }

    pub fn height(&self) -> u32 {
        self.video.height
    }

    /// 根据 time_base 把时间戳转为时长。
    pub fn timestamp_to_duration(&self, ts: u64) -> Duration {
        let numer = self.video.time_base_numer as u128;
        let denom = self.video.time_base_denom.max(1) as u128;
        let nanos = ts as u128 * numer * 1_000_000_000u128 / denom;
        Duration::from_nanos(nanos.min(u64::MAX as u128) as u64)
    }

    pub fn seek_seconds(&mut self, seconds: f64) -> Result<(), SparkError> {
        self.reader.seek_seconds(seconds).map_err(SparkError::from)
    }

    /// 下一视频包；到 EOS 返回 `None`。
    pub fn next_video_packet(&mut self) -> Result<Option<MediaPacket>, SparkError> {
        loop {
            match self
                .reader
                .next_packet(Some(self.video.track_id))
                .map_err(SparkError::from)?
            {
                None => return Ok(None),
                Some(p) if p.kind == PacketKind::Video || p.track_id == self.video.track_id => {
                    return Ok(Some(p));
                }
                Some(_) => continue,
            }
        }
    }

    /// 下一任意包（视频或同容器音频），便于上层交错调度。
    pub fn next_packet(&mut self) -> Result<Option<MediaPacket>, SparkError> {
        self.reader.next_packet(None).map_err(SparkError::from)
    }
}

/// 占位：尚未像素解码的视频帧句柄（携带压缩包）。
#[derive(Debug, Clone)]
pub struct EncodedFrame {
    pub pts: Duration,
    pub packet: MediaPacket,
}

impl VideoClip {
    pub fn next_encoded_frame(&mut self) -> Result<Option<EncodedFrame>, SparkError> {
        let Some(packet) = self.next_video_packet()? else {
            return Ok(None);
        };
        let pts = self.timestamp_to_duration(packet.ts);
        Ok(Some(EncodedFrame { pts, packet }))
    }
}
