# `spark-napi`

本目录是 Rust crate `spark-napi` 旁的 **构建辅助** npm 元数据（`private: true`），不是给业务直接安装的发布面。

真正发布给用户的是：

- `@game-gpt/sparkle-engine`（JS API + CLI `spark`）
- `@game-gpt/sparkle-engine-<os-cpu>`（`.node` 平台袋）

## 构建

在 SparkEngine 仓库根：

```bash
node scripts/build/napi.mjs --release
# 或
pnpm run build:napi
```

等价于 `cargo build -p spark-napi --features node`，再把产物装进 `projects/platforms/native/sparkle-engine-<short>/`。

本包 `package.json` 的 `scripts.build` 会转到上述脚本；`napi` 字段描述 triple 列表，供工具链参考。

## 相关

- Rust API / feature：`projects/bindings/spark-napi/readme.md`
- 宿主加载逻辑：`projects/hosts/sparkle-engine/README.md`
