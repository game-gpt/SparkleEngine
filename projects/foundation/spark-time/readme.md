# spark-time

Frame clock `Clock`: feed wall-clock seconds, returns how many simulation steps to run this frame.

```rust
use spark_time::Clock;

let mut clock = Clock::fixed(1.0 / 60.0, 5);
let steps = clock.begin_frame(real_dt);
for _ in 0..steps {
    simulate(clock.delta_seconds);
}
```

- `Clock::fixed(fixed_dt, max_substeps)`: accumulator yields `0..=max_substeps` steps, each with `delta_seconds == fixed_dt`
- `Clock::variable()`: when not paused, returns `1` per frame; `delta_seconds` is scaled real dt
- `set_paused(true)` / `set_scale(s)`: pause or time scale (`scale == 0` does not advance)
- Public fields: `elapsed_seconds`, `delta_seconds`, `scale`, `paused`

`Default` equals `fixed(1/60, 5)`. Main loop is driven by `spark-engine` or the host calling `begin_frame`.

```bash
cargo test -p spark-time
```
