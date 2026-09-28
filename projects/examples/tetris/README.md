# tetris

俄罗斯方块：棋盘权威在 Rust（`Board` / `PieceKind` / `TetrisApp`），可挂 Valkyrie HUD 元数据。入口同样是 `run_game`。

```bash
cargo run -p tetris
```

下落、旋转、消行从 `TetrisApp` 与 `Board` 改起。Studio 中识别为 Hybrid 工程。

```bash
cargo test -p tetris
```
