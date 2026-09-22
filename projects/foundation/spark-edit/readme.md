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