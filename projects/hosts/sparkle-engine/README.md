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
# 在游戏项目目录（含 package.json）
spark studio
spark studio --cwd path/to/game

spark info
```

`spark studio` 读取当前（或 `--cwd`）目录的 `package.json`，打开该 npm 游戏项目的编辑器。**不是** Launcher / 项目选择器。

## 开发

在仓库根：

```bash
pnpm install
pnpm run build:ts
node scripts/build/napi.mjs
pnpm exec spark info
pnpm exec spark studio
```
