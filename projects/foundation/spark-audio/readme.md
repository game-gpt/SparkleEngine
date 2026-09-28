# spark-audio

Audio output and playback queue. PCM decode uses `spark-media`; this crate manages device, `Tone`, and `AudioQueue::flush`.

```rust
use spark_audio::{AudioBus, AudioQueue, Tone};

let bus = AudioBus::silent();
// or AudioBus::try_open() — falls back to silent if device open fails

let mut queue = AudioQueue::new();
queue.play_tone(Tone::new(440.0, 120, 0.3));
queue.play_file("sfx.wav");
queue.flush(&bus, None);
```

`flush` can take `Some(&mut Vec<SparkError>)` to collect file playback failures. Also `PcmStream` over `PcmAudio`, and `start_pcm`. Re-exports `AudioDecoder` / `PcmAudio`.

```bash
cargo test -p spark-audio
```
