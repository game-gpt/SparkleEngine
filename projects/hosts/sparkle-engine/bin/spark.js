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
  spark studio [project-path] [--safe-mode]

Sparkle Engine CLI（@game-gpt/sparkle-engine）
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

function runStudio(args) {
  const bin = findStudioBinary();
  if (!bin) {
    console.error(
      "找不到 spark-studio 二进制。请先在仓库根执行：cargo build -p spark-studio",
    );
    process.exit(1);
  }
  const result = spawnSync(bin, args, {
    stdio: "inherit",
    windowsHide: true,
    env: process.env,
  });
  if (result.error) {
    console.error(result.error);
    process.exit(1);
  }
  process.exit(result.status ?? 1);
}

function runInfo() {
  // 延迟加载，避免无 info 时也强依赖已构建的 dist / 原生插件。
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
