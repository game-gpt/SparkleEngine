/**
 * 统一格式化：Biome（TS/JS/JSON）+ `cargo fmt`（Rust）。
 *
 * Usage:
 *   node scripts/format.mjs           # 写入
 *   node scripts/format.mjs --check   # 仅检查，不改文件
 */

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const check = process.argv.includes("--check");

/**
 * @param {string} command
 * @param {string[]} args
 * @param {string} label
 */
function run(command, args, label) {
    console.log(`→ ${label}: ${command} ${args.join(" ")}`);
    const result = spawnSync(command, args, {
        cwd: root,
        stdio: "inherit",
        shell: false,
        windowsHide: true,
        env: process.env,
    });
    if (result.error) {
        console.error(result.error);
        process.exit(1);
    }
    if (result.status !== 0) {
        process.exit(result.status ?? 1);
    }
}

const biomeBin = path.join(root, "node_modules", "@biomejs", "biome", "bin", "biome");

if (check) {
    run("node", [biomeBin, "format", "."], "biome format (check)");
    run("cargo", ["fmt", "--all", "--", "--check"], "cargo fmt (check)");
} else {
    run("node", [biomeBin, "format", "--write", "."], "biome format");
    run("cargo", ["fmt", "--all"], "cargo fmt");
}

console.log(check ? "fmt check ok" : "fmt ok");
