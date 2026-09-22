# ping-pong

纯 Rust 乒乓：`PingPongGame` 实现 `GameHost`，经 `spark_engine::run_game` 开窗。

```bash
cargo run -p ping-pong
```

`main` 使用：

```rust
run_game(
    WindowConfig {
        title: "ping-pong".into(),
        width: 960,
        height: 540,
        clear_color: [0.05, 0.07, 0.10, 1.0],
    },
    PingPongGame::new(),
)?;
```

| 内容        | 文件                           |
|-------------|--------------------------------|
| 球速 / 发球 | `src/ball.rs`（`Ball::serve`） |
| 挡板        | `src/paddle.rs`                |
| 碰撞与得分  | `src/game.rs`                  |
| 窗口        | `src/main.rs`                  |

无音频、无脚本模组。Studio 可按 Rust 工程打开。
