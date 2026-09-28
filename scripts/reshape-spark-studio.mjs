/**
 * Spark Studio 架构硬切断：shell.rs / state.rs 单体 → ui / layout / state 分层。
 *
 * 无兼容层：删除根级 `shell.rs`、`state.rs`，不在 lib.rs 做 re-export。
 *
 * node scripts/reshape-spark-studio.mjs
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const crate = path.join(root, "projects/gameplay/spark-studio");

/** @type {string[]} */
const requiredDirs = ["src/ui", "src/layout", "src/state"];

/** @type {string[]} */
const requiredFiles = [
    "src/lib.rs",
    "src/app.rs",
    "src/ui/mod.rs",
    "src/ui/shell.rs",
    "src/ui/chrome.rs",
    "src/ui/top_bar.rs",
    "src/ui/scene_toolbar.rs",
    "src/ui/hierarchy.rs",
    "src/ui/viewport.rs",
    "src/ui/inspector.rs",
    "src/ui/asset_browser.rs",
    "src/ui/console.rs",
    "src/ui/problems.rs",
    "src/ui/bottom_dock.rs",
    "src/ui/status_bar.rs",
    "src/layout/mod.rs",
    "src/layout/dock.rs",
    "src/layout/splitter.rs",
    "src/layout/persistence.rs",
    "src/layout/preset.rs",
    "src/state/mod.rs",
    "src/state/commands.rs",
    "src/state/panels.rs",
    "src/state/play_mode.rs",
    "src/state/tool.rs",
    "src/state/selection.rs",
    "src/state/workspace.rs",
];

/** @type {string[]} */
const obsoleteFiles = ["src/shell.rs", "src/state.rs"];

function abs(rel) {
    return path.join(crate, rel);
}

let created = 0;
let deleted = 0;
let failed = 0;

for (const dir of requiredDirs) {
    const dirPath = abs(dir);
    if (!fs.existsSync(dirPath)) {
        fs.mkdirSync(dirPath, { recursive: true });
        console.log(`mkdir ${dir}`);
        created += 1;
    }
}

for (const rel of requiredFiles) {
    const filePath = abs(rel);
    if (!fs.existsSync(filePath)) {
        console.error(`missing required ${rel}`);
        failed += 1;
    }
}

for (const rel of obsoleteFiles) {
    const filePath = abs(rel);
    if (!fs.existsSync(filePath)) {
        continue;
    }
    fs.unlinkSync(filePath);
    console.log(`deleted ${rel}`);
    deleted += 1;
}

if (failed > 0) {
    console.error(`reshape-spark-studio: FAILED missing=${failed}`);
    process.exit(1);
}

console.log(`reshape-spark-studio: created_dirs=${created} deleted=${deleted}`);
console.log("手动后续：cargo build -p spark-studio && cargo test -p spark-studio");
