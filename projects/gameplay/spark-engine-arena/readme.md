# spark-engine-arena

Spark **竞技场**（双摇杆生存 / 几何射击）特异化壳：**不含**具体敌种、Boss 剧本或 Roguelike 数值。

## 能力

- [`KineticPool`](src/pool.rs)：位置 + 速度 + 半径 + 标签的对象池（玩家弹、敌人、拾取物等由游戏定义 `tag` / `layer`）
- [`WaveTimeline`](src/wave.rs)：按秒触发的生成事件队列（只输出 `tag`，不解释语义）
- [`CircleGrid`](src/spatial.rs)：均匀网格圆邻近查询
- [`RunClock`](src/clock.rs)：单调关卡秒表

## 边界

与 [`spark-engine-stg`](../spark-engine-stg/) 相对：STG 面向弹幕→玩家；Arena 面向玩家↔敌人↔环境的双向动能体。

游戏层（如 GeometryLab `prism-raid`）负责形状—AI 映射、Boss 改规则与 Roguelike 成长。
