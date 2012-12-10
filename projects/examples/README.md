# examples

Studio 三种项目形态的验收矩阵（小写目录）：

| 目录 | `spark.kind` | 说明 |
|------|--------------|------|
| `ping-pong/` | `rust` | Cargo + Assets，无 Scripts |
| `snake/` | `valkyrie` | 仅 Assets/Scripts，无 Cargo |
| `tetris/` | `hybrid` | Cargo + Scripts |

```powershell
pnpm --filter ping-pong studio
pnpm --filter snake studio
pnpm --filter tetris studio
```

依赖统一为 `@game-gpt/sparkle-engine`（**没有** `spark-studio` npm 包）。
