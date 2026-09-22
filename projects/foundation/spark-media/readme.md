# spark-media

基于 Symphonia 的容器探测、解复用与音频 PCM 解码。

```rust
use spark_media::{probe_path, AudioDecoder};

let info = probe_path("clip.ogg")?;
// MediaReader 按包泵送；AudioDecoder 解出 PcmAudio
```

公开类型包括 `MediaInfo`、`MediaReader`、`MediaPacket`、`AudioDecoder`、`PcmAudio`、`MediaError`。`spark-audio` 与
`spark-video` 建立在这层之上。

```bash
cargo test -p spark-media
```
