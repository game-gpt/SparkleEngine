# spark-edit

Edit runtime: asset / prefab edit transactions. The `spark-shell` binary is forwarded by `spark shell`.

Currently executable formats:

1. **VON `ops` plan** (`EditPlan`)
2. **Edit profile host calls** (`profile = "spark-edit-1"` + `calls`) → lowered to `EditOp`

The Sparkle Script Edit profile will bind the same [`EditSession`].

```bash
cargo build -p spark-edit
cargo run -p spark-edit --bin spark-shell -- --dry-run --json --code "ops = []"
# apply requires write capability (local CLI grants project-edit on --apply by default)
pnpm exec spark shell --apply --capability project-edit --code "ops = []"
```

MCP: `spark mcp` defaults to `capabilities = ["read-project"]`; `mode=apply` must explicitly include `project-edit` / `write-assets`.

```bash
cargo test -p spark-edit
```
