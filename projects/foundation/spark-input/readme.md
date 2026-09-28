# spark-input

Per-frame input snapshot `Input`, plus engine-owned key codes `Key` / `MouseBtn` / `ButtonState`. Also action mapping `ActionMap`.

Window backend writes:

```rust
input.on_key(key, pressed);
input.on_mouse_button(btn, pressed);
input.on_cursor(x, y);
input.on_wheel(dx, dy);
input.on_text(ch);
```

Games read `key_down` / `key_pressed` / `key_released`, `mouse_pos`, etc. from `frame.input` in `GameHost::update`. Call `begin_frame` at frame end to clear edge state.

winit and similar types do not leak into game code; mapping happens in `spark-renderer-wgpu` (or the napi host).

```bash
cargo test -p spark-input
```
