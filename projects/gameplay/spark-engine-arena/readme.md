# spark-engine-arena

Spark **arena** (twin-stick survival / geometric shooter) specialization shell: **no** concrete enemy types, boss scripts, or roguelike tuning.

## Capabilities

- [`KineticPool`](src/pool.rs): object pool with position, velocity, radius, and tag (games define `tag` / `layer` for player shots, enemies, pickups, etc.)
- [`WaveTimeline`](src/wave.rs): spawn event queue triggered by seconds (outputs `tag` only, no semantics)
- [`CircleGrid`](src/spatial.rs): uniform-grid circle neighbor queries
- [`RunClock`](src/clock.rs): monotonic stage timer

## Boundary

Compared to [`spark-engine-stg`](../spark-engine-stg/): STG is bullet → player; Arena is player ↔ enemy ↔ environment kinetic bodies.

The game layer (e.g. GeometryLab `prism-raid`) owns shape–AI mapping, boss rule changes, and roguelike progression.
