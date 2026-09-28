# spark-shader

WGSL 源码与内建管线标识，编译为 wgpu `ShaderModule`。

```rust
use spark_shader::{BuiltinShader, create_builtin, validate_source, ShaderSource};

validate_source(&ShaderSource::from(BuiltinShader::SolidQuad))?;
let module = create_builtin(&device, BuiltinShader::SolidQuad);
```

也可用 `create_module(device, source)`。空源码等失败返回 `SparkError`。渲染后端消费编译结果，不在业务代码里内嵌大段 WGSL。

```bash
cargo test -p spark-shader
```
