# 平台 npm 袋

对齐 `ra2.exe` 布局：

```text
projects/hosts/sparkle-engine/              # @game-gpt/sparkle-engine（唯一 bin：spark）
projects/platforms/native/sparkle-engine-*/ # 原生 .node 袋
projects/platforms/wasm/sparkle-engine-unknown-wasm32/
```

仓库根使用 **pnpm workspace**（见 `pnpm-workspace.yaml`）。**禁止**再立 `packages/`。
