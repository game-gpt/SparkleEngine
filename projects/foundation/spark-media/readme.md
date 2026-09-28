# spark-media

Container probing, demuxing, and audio PCM decode on top of Symphonia.

```rust
use spark_media::{probe_path, AudioDecoder};

let info = probe_path("clip.ogg")?;
// MediaReader pumps packets; AudioDecoder yields PcmAudio
```

Public types include `MediaInfo`, `MediaReader`, `MediaPacket`, `AudioDecoder`, `PcmAudio`, `MediaError`. `spark-audio` and `spark-video` build on this layer.

```bash
cargo test -p spark-media
```
