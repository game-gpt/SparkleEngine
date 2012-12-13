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
  spark studio [--cwd <project-dir>] [--safe-mode]

Sparkle Engine CLI（@game-gpt/sparkle-engine）

studio 读取当前（或 --cwd）目录的 package.json，打开该 npm 游戏项目的编辑器。
不是 Launcher，不提供项目选择列表。
`);
}

/** SparkEngine 仓库根：projects/hosts/sparkle-engine/bin → ../../../.. */
function engineRoot() {
  return path.resolve(__dirname, "../../../..");
}

function findStudioBinary() {
  const env =
    (typeof process.env.SPARK_STUDIO_BIN === "string" &&
      process.env.SPARK_STUDIO_BIN.trim()) ||
    "";
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

/** 预检：cwd 必须有 package.json（与 Studio 二进制一致）。 */
function resolveStudioCwd(args) {
  let cwd = process.cwd();
  const out = [];
  for (let i = 0; i < args.length; i += 1) {
    const a = args[i];
    if (a === "--cwd" && args[i + 1]) {
      cwd = path.resolve(args[i + 1]);
      out.push("--cwd", cwd);
      i += 1;
      continue;
    }
    if (a.startsWith("--cwd=")) {
      cwd = path.resolve(a.slice("--cwd=".length));
      out.push(`--cwd=${cwd}`);
      continue;
    }
    if (!a.startsWith("-")) {
      cwd = path.resolve(a);
      out.push("--cwd", cwd);
      continue;
    }
    out.push(a);
  }
  const pkg = path.join(cwd, "package.json");
  if (!fs.existsSync(pkg)) {
    console.error(
      `当前目录不是 Spark 游戏项目。\n未找到 package.json（${cwd}）。\n请在已安装 @game-gpt/sparkle-engine 的项目目录中运行 spark studio。`,
    );
    process.exit(2);
  }
  return out.length ? out : ["--cwd", cwd];
}

function runStudio(args) {
  const bin = findStudioBinary();
  if (!bin) {
    console.error(
      "找不到 spark-studio 二进制。请先在仓库根执行：cargo build -p spark-studio",
    );
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
