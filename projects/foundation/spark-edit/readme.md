# spark-edit

Edit Runtime：资源 / Prefab 编辑事务。`spark-shell` 二进制由 `spark shell` 转发。

当前可执行格式：

1. **VON `ops` 计划**（`EditPlan`）
2. **Edit profile 宿主调用**（`profile = "spark-edit-1"` + `calls`）→ 降级为 `EditOp`

Sparkle Script Edit profile 将绑定同一 [`EditSession`]。

```bash
cargo build -p spark-edit
cargo run -p spark-edit --bin spark-shell -- --dry-run --json --code "ops = []"
# apply 需写能力（本地 CLI 默认在 --apply 时授予 project-edit）
pnpm exec spark shell --apply --capability project-edit --code "ops = []"
```

MCP：`spark mcp` 默认 `capabilities = ["read-project"]`；`mode=apply` 必须显式带 `project-edit` / `write-assets`。

```bash
cargo test -p spark-edit
```
