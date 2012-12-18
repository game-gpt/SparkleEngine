# examples

Studio 三种项目形态的验收矩阵（小写目录）：

| 目录 | `spark.kind` | 说明 |
|------|--------------|------|
| `ping-pong/` | `rust` | Cargo + `assets/`，无 `scripts` |
| `snake/` | `valkyrie` | `assets/scripts` + native 试玩宿主（VM Play 前） |
| `tetris/` | `hybrid` | Cargo + `scripts` |

```powershell
# 直接开玩（不要只开编辑器）
pnpm --filter ping-pong play
pnpm --filter snake play
pnpm --filter tetris play

# 或在对应示例目录
spark run

# 编辑器：Play 会嵌入同一套对局
pnpm --filter ping-pong studio
```

依赖统一为 `@game-gpt/sparkle-engine`（**没有** `spark-studio` npm 包）。
