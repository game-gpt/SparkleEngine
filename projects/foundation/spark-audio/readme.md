# spark-audio

音频输出与播放队列。解码 PCM 用 `spark-media`；本 crate 管设备、`Tone` 与 `AudioQueue::flush`。

```rust
use spark_audio::{AudioBus, AudioQueue, Tone};

let bus = AudioBus::silent();
// 或 AudioBus::try_open() —— 打不开设备时内部降为静默

let mut queue = AudioQueue::new();
queue.play_tone(Tone::new(440.0, 120, 0.3));
queue.play_file("sfx.wav");
queue.flush(&bus, None);
```

`flush` 可传入 `Some(&mut Vec<SparkError>)` 收集文件播放失败。另有 `PcmStream` 遍历 `PcmAudio`，以及 `start_pcm`。再导出
`AudioDecoder` / `PcmAudio`。

```bash
cargo test -p spark-audio
```
