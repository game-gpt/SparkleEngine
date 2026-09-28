# spark-video

Video track demux and compressed packet pumping (via `spark-media`). No pixel decode.

```rust
use spark_video::VideoClip;

let mut clip = VideoClip::open("clip.mp4")?;
// seek_seconds / next_video_packet / next_encoded_frame
```

`EncodedFrame` carries compressed data and timestamps. Errors are `SparkError`. Display path connects frames on the render / hardware-decode side.

```bash
cargo test -p spark-video
```
