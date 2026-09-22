#!/usr/bin/env node
"use strict";

/**
 * 全项目唯一 npm bin：`spark`。
 * Rust crate `spark-engine` / `spark-studio` 以及其它 npm 包不得再声明 bin。
 */

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

function usage() {
    console.log(`Usage:
  spark info
  spark run [--cwd <project-dir>]
  spark studio [--cwd <project-dir>] [--safe-mode] [--play]
  spark shell --path <plan.edit.von> [--mode check|dry-run|apply] [--cwd <dir>] [--json]
  spark shell --code <von-text> [--check|--dry-run|--apply] [--json]
  spark script --path <plan.edit.von> ...

Sparkle Engine CLI（@game-gpt/sparkle-engine）

run     读取 package.json，启动游戏（rust/hybrid → cargo run -p <runTarget>；
        无 runTarget 的 script → spark-studio --play）。
studio  打开该 npm 游戏项目的编辑器。--play 则跳过壳直接进对局。
shell   运行 Edit Runtime（VON 编辑计划）。script 为同义入口。
`);
}

/** SparkEngine 仓库根：projects/hosts/sparkle-engine/bin → ../../../.. */
function engineRoot() {
    return path.resolve(__dirname, "../../../..");
}

function findStudioBinary() {
    const env = (typeof process.env.SPARK_STUDIO_BIN === "string" && process.env.SPARK_STUDIO_BIN.trim()) || "";
    if (env) {
        const abs = path.resolve(env);
        if (!fs.existsSync(abs)) {
            throw new Error(`SPARK_STUDIO_BIN 指向的文件不存在：${abs}`);
        }
        return abs;
    }

    const root = engineRoot();
    const name = process.platform === "win32" ? "spark-studio.exe" : "spark-studio";
    for (const profile of ["release", "debug"]) {
        const candidate = path.join(root, "target", profile, name);
        if (fs.existsSync(candidate)) return candidate;
    }
    return null;
}

function findShellBinary() {
    const env = (typeof process.env.SPARK_SHELL_BIN === "string" && process.env.SPARK_SHELL_BIN.trim()) || "";
    if (env) {
        const abs = path.resolve(env);
        if (!fs.existsSync(abs)) {
            throw new Error(`SPARK_SHELL_BIN 指向的文件不存在：${abs}`);
        }
        return abs;
    }

    const root = engineRoot();
    const name = process.platform === "win32" ? "spark-shell.exe" : "spark-shell";
    for (const profile of ["release", "debug"]) {
        const candidate = path.join(root, "target", profile, name);
        if (fs.existsSync(candidate)) return candidate;
    }
    return null;
}

function parseCwd(args) {
    let cwd = process.cwd();
    const rest = [];
    for (let i = 0; i < args.length; i += 1) {
        const a = args[i];
        if (a === "--cwd" && args[i + 1]) {
            cwd = path.resolve(args[i + 1]);
            i += 1;
            continue;
        }
        if (a.startsWith("--cwd=")) {
            cwd = path.resolve(a.slice("--cwd=".length));
            continue;
        }
        rest.push(a);
    }
    return { cwd, rest };
}

/** 预检：cwd 必须有 package.json（与 Studio 二进制一致）。 */
function resolveStudioCwd(args) {
    const { cwd, rest } = parseCwd(args);
    const pkg = path.join(cwd, "package.json");
    if (!fs.existsSync(pkg)) {
        console.error(
            `当前目录不是 Spark 游戏项目。\n未找到 package.json（${cwd}）。\n请在已安装 @game-gpt/sparkle-engine 的项目目录中运行 spark studio。`,
        );
        process.exit(2);
    }
    const out = ["--cwd", cwd, ...rest];
    return out;
}

function readSparkField(cwd) {
    const pkgPath = path.join(cwd, "package.json");
    const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
    return {
        name: pkg.name || path.basename(cwd),
        spark: pkg.spark || {},
    };
}

function runStudio(args) {
    const bin = findStudioBinary();
    if (!bin) {
        console.error("找不到 spark-studio 二进制。请先在仓库根执行：cargo build -p spark-studio");
        process.exit(1);
    }
    const forwarded = resolveStudioCwd(args);
    const result = spawnSync(bin, forwarded, {
        stdio: "inherit",
        windowsHide: true,
        env: process.env,
        cwd: process.cwd(),
    });
    if (result.error) {
        console.error(result.error);
        process.exit(1);
    }
    process.exit(result.status ?? 1);
}

function runShell(args) {
    const bin = findShellBinary();
    if (!bin) {
        console.error("找不到 spark-shell 二进制。请先在仓库根执行：cargo build -p spark-edit");
        process.exit(1);
    }
    const result = spawnSync(bin, args, {
        stdio: "inherit",
        windowsHide: true,
        env: process.env,
        cwd: process.cwd(),
    });
    if (result.error) {
        console.error(result.error);
        process.exit(1);
    }
    process.exit(result.status ?? 1);
}

function runGame(args) {
    const { cwd, rest } = parseCwd(args);
    const pkgPath = path.join(cwd, "package.json");
    if (!fs.existsSync(pkgPath)) {
        console.error(`当前目录不是 Spark 游戏项目。\n未找到 package.json（${cwd}）。`);
        process.exit(2);
    }
    const { name, spark } = readSparkField(cwd);
    const runTarget = spark.runTarget || null;
    const root = engineRoot();

    if (runTarget) {
        const result = spawnSync("cargo", ["run", "-p", runTarget, ...rest], {
            stdio: "inherit",
            windowsHide: true,
            env: process.env,
            cwd: root,
        });
        if (result.error) {
            console.error(result.error);
            process.exit(1);
        }
        process.exit(result.status ?? 1);
    }

    // 无 runTarget：走 Studio --play（嵌入对局）
    console.log(`spark run: 项目 ${name} 无 spark.runTarget，使用 spark-studio --play`);
    runStudio(["--cwd", cwd, "--play", ...rest]);
}

function runInfo() {
    const { loadSpark } = require("../dist/index.js");
    const info = loadSpark().info();
    console.log(JSON.stringify(info, null, 2));
}

function main() {
    const argv = process.argv.slice(2);
    const cmd = argv[0];

    if (!cmd || cmd === "-h" || cmd === "--help") {
        usage();
        return;
    }

    if (cmd === "studio") {
        runStudio(argv.slice(1));
        return;
    }

    if (cmd === "shell" || cmd === "script") {
        runShell(argv.slice(1));
        return;
    }

    if (cmd === "run") {
        runGame(argv.slice(1));
        return;
    }

    if (cmd === "info") {
        try {
            runInfo();
        } catch (err) {
            console.error(err instanceof Error ? err.message : err);
            process.exit(1);
        }
        return;
    }

    console.error(`未知命令：${cmd}`);
    usage();
    process.exit(1);
}

main();
