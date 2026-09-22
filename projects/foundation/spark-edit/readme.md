# spark-edit

Edit Runtime：资源 / Prefab 编辑事务。`spark-shell` 二进制由 `spark shell` 转发。

当前可执行格式为 **VON 编辑计划**（`EditPlan.ops`）。Sparkle Script Edit profile 将绑定同一 [`EditSession`]。

```bash
cargo build -p spark-edit
cargo run -p spark-edit --bin spark-shell -- --dry-run --json --code "ops = []"
# 或经 npm：pnpm exec spark shell --dry-run --json --code "ops = []"
```

`spark shell` 与 `spark script` 同义，转发到 `spark-shell`。

`--mode check|dry-run|apply` 可用 `--check` / `--dry-run` / `--apply` 简写。

MCP / Agent 应只暴露少量工具，内部调用同一入口：

- Rust：`spark_edit::run_edit_plan`
- N-API / JS：`loadSpark().runEditPlan(root, mode, von)`（需重建 napi）
- CLI：`spark shell --code … --json`
- 极简 MCP stdio：`node projects/hosts/sparkle-engine/bin/spark-mcp.js`（工具名 `spark_script`）

```bash
cargo test -p spark-edit
```
