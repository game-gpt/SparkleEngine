# snake

贪吃蛇：`SnakeApp` 实现 `GameHost`，`run_game` 开窗。方向键 / WASD 移动，R 重开，Esc 退出。

```bash
cargo run -p snake
```

| 内容                          | 位置               |
|-------------------------------|--------------------|
| 格子 `CELL` / `COLS` / `ROWS` | `src/lib.rs` 顶部  |
| 移动与得分                    | `SnakeApp::update` |
| 绘制与 HUD                    | `SnakeApp::draw`   |
| 窗口尺寸                      | `src/main.rs`      |

工程目录可带 Valkyrie 元数据；本 crate 是 native 试玩宿主。
