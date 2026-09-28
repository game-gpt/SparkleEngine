# spark-input

每帧输入快照 `Input`，以及引擎自有键码 `Key` / `MouseBtn` / `ButtonState`。另有动作映射 `ActionMap`。

窗口后端写入：

```rust
input.on_key(key, pressed);
input.on_mouse_button(btn, pressed);
input.on_cursor(x, y);
input.on_wheel(dx, dy);
input.on_text(ch);
```

游戏在 `GameHost::update` 里读取 `frame.input` 的 `key_down` / `key_pressed` / `key_released`、`mouse_pos` 等。帧末调用
`begin_frame` 清除边沿状态。

winit 等类型不会漏到游戏代码；映射在 `spark-renderer-wgpu`（或 napi 宿主）完成。

```bash
cargo test -p spark-input
```
