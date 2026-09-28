# spark-shader

WGSL sources and built-in pipeline identifiers, compiled to wgpu `ShaderModule`.

```rust
use spark_shader::{BuiltinShader, create_builtin, validate_source, ShaderSource};

validate_source(&ShaderSource::from(BuiltinShader::SolidQuad))?;
let module = create_builtin(&device, BuiltinShader::SolidQuad);
```

Also `create_module(device, source)`. Empty source and similar failures return `SparkError`. Rendering backend consumes compiled output; do not embed large WGSL blobs in game code.

```bash
cargo test -p spark-shader
```
