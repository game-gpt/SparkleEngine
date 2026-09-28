# spark-time

帧时钟 `Clock`：喂入墙钟秒数，返回本帧应执行的仿真步数。

```rust
use spark_time::Clock;

let mut clock = Clock::fixed(1.0 / 60.0, 5);
let steps = clock.begin_frame(real_dt);
for _ in 0..steps {
    simulate(clock.delta_seconds);
}
```

- `Clock::fixed(fixed_dt, max_substeps)`：累积器吐出 `0..=max_substeps` 步，每步 `delta_seconds == fixed_dt`
- `Clock::variable()`：未暂停时每帧返回 `1`，`delta_seconds` 为缩放后的真实 dt
- `set_paused(true)` / `set_scale(s)`：暂停或缩放（`scale == 0` 不推进）
- 公开字段：`elapsed_seconds`、`delta_seconds`、`scale`、`paused`

`Default` 等于 `fixed(1/60, 5)`。主循环由 `spark-engine` 或宿主调用 `begin_frame`。

```bash
cargo test -p spark-time
```
