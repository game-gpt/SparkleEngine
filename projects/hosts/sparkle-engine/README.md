# `@game-gpt/sparkle-engine`

Sparkle 元引擎的 **TypeScript 宿主包**（`projects/hosts/sparkle-engine`）。

全仓库 **唯一** npm CLI：`spark`（本包 `bin`）。Rust crate `spark-engine` / `spark-studio` 及其它 npm 包 **不得** 再声明 `bin`。

## 安装

```bash
pnpm add @game-gpt/sparkle-engine
```

`optionalDependencies` 会按 `os` / `cpu` 拉取匹配的原生平台袋。浏览器 / WASI 请显式依赖 `@game-gpt/sparkle-engine-unknown-wasm32`。

## CLI

```bash
spark info
spark studio [project-path]
```

`spark studio` 启动已构建的 `spark-studio` 二进制（默认查找仓库 `target/{release,debug}/`；可用 `SPARK_STUDIO_BIN` 覆盖）。

## 开发

在仓库根：

```bash
pnpm install
pnpm run build:ts
node scripts/build/napi.mjs
pnpm exec spark info
pnpm exec spark studio
```
