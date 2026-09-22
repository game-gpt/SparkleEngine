# spark-video

视频轨解复用与压缩包泵送（经 `spark-media`）。不做像素解码。

```rust
use spark_video::VideoClip;

let mut clip = VideoClip::open("clip.mp4")?;
// seek_seconds / next_video_packet / next_encoded_frame
```

`EncodedFrame` 携带压缩数据与时间戳。错误为 `SparkError`。显示路径由渲染 / 硬件解码侧接好帧。

```bash
cargo test -p spark-video
```
