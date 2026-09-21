//! 解复用：按包拉取音视频轨（Symphonia 0.6）。

use std::{fs::File, path::Path};

use symphonia::{
    core::{
        formats::{FormatOptions, FormatReader, SeekMode, SeekTo, probe::Hint},
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::MetadataOptions,
        units::Time,
    },
    default::get_probe,
};

use crate::{
    MediaError,
    probe::{AudioTrackInfo, MediaInfo, VideoTrackInfo, classify_tracks},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketKind {
    Audio,
    Video,
    Other,
}

/// 解复用后的原始包（时间戳单位为轨 time_base）。
#[derive(Debug, Clone)]
pub struct MediaPacket {
    pub track_id: u32,
    pub kind: PacketKind,
    pub ts: u64,
    pub dur: Option<u64>,
    pub data: Vec<u8>,
}

/// 打开中的媒体读者（持有 Symphonia `FormatReader`）。
pub struct MediaReader {
    format: Box<dyn FormatReader>,
    info: MediaInfo,
}

impl MediaReader {
    pub fn open_path(path: impl AsRef<Path>) -> Result<Self, MediaError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| MediaError::open_path(path, &e))?;
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        Self::open_mss(MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default()), hint)
    }

    pub fn open_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, MediaError> {
        let cursor = std::io::Cursor::new(bytes.into());
        Self::open_mss(MediaSourceStream::new(Box::new(cursor), MediaSourceStreamOptions::default()), Hint::new())
    }

    fn open_mss(mss: MediaSourceStream<'static>, hint: Hint) -> Result<Self, MediaError> {
        let format: Box<dyn FormatReader> = get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())?;
        let info = classify_tracks(format.tracks());
        Ok(Self { format, info })
    }

    pub fn info(&self) -> &MediaInfo {
        &self.info
    }

    pub fn default_audio(&self) -> Option<&AudioTrackInfo> {
        self.info.default_audio()
    }

    pub fn default_video(&self) -> Option<&VideoTrackInfo> {
        self.info.default_video()
    }

    /// 拉取下一包；`track_filter` 为 `None` 时接受任意轨。
    pub fn next_packet(&mut self, track_filter: Option<u32>) -> Result<Option<MediaPacket>, MediaError> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => return Ok(None),
                Err(symphonia::core::errors::Error::ResetRequired) => continue,
                Err(e) => return Err(e.into()),
            };
            if let Some(id) = track_filter {
                if packet.track_id != id {
                    continue;
                }
            }
            let track_id = packet.track_id;
            let kind = self.kind_of(track_id);
            let ts = packet.pts.get().max(0) as u64;
            let dur = packet.dur.get();
            return Ok(Some(MediaPacket { track_id, kind, ts, dur: Some(dur).filter(|&d| d > 0), data: packet.data.into_vec() }));
        }
    }

    /// 按秒粗略寻道（依赖容器 seek 支持）。
    pub fn seek_seconds(&mut self, seconds: f64) -> Result<(), MediaError> {
        let track_id = self.info.default_video().map(|v| v.track_id).or_else(|| self.info.default_audio().map(|a| a.track_id));
        let time = Time::try_from_secs_f64(seconds).unwrap_or(Time::ZERO);
        let seek = SeekTo::Time { time, track_id };
        self.format.seek(SeekMode::Coarse, seek)?;
        Ok(())
    }

    fn kind_of(&self, track_id: u32) -> PacketKind {
        use crate::probe::TrackInfo;
        for t in &self.info.tracks {
            match t {
                TrackInfo::Audio(a) if a.track_id == track_id => return PacketKind::Audio,
                TrackInfo::Video(v) if v.track_id == track_id => return PacketKind::Video,
                TrackInfo::Other { track_id: id, .. } if *id == track_id => {
                    return PacketKind::Other;
                }
                _ => {}
            }
        }
        PacketKind::Other
    }
}
