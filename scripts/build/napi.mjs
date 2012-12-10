/**
 * 构建 `spark-napi` → 当前平台 `@game-gpt/sparkle-engine-<short>`：
 *   编译后把 `.node` 装进 `projects/platforms/native/sparkle-engine-<short>/`。
 *
 * Usage: node scripts/build/napi.mjs [--release]
 */

import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const release = process.argv.includes("--release");
const profile = release ? "release" : "debug";

/** @returns {{ triple: string, short: string, os: string[], cpu: string[] }} */
function platformInfo() {
  const { platform, arch } = process;
  if (platform === "win32" && arch === "x64") {
    return { triple: "win32-x64-msvc", short: "win32-x64", os: ["win32"], cpu: ["x64"] };
  }
  if (platform === "win32" && arch === "arm64") {
    return { triple: "win32-arm64-msvc", short: "win32-arm64", os: ["win32"], cpu: ["arm64"] };
  }
  if (platform === "darwin" && arch === "arm64") {
    return { triple: "darwin-arm64", short: "darwin-arm64", os: ["darwin"], cpu: ["arm64"] };
  }
  if (platform === "darwin" && arch === "x64") {
    return { triple: "darwin-x64", short: "darwin-x64", os: ["darwin"], cpu: ["x64"] };
  }
  if (platform === "linux" && arch === "x64") {
    return { triple: "linux-x64-gnu", short: "linux-x64", os: ["linux"], cpu: ["x64"] };
  }
  if (platform === "linux" && arch === "arm64") {
    return { triple: "linux-arm64-gnu", short: "linux-arm64", os: ["linux"], cpu: ["arm64"] };
  }
  const triple = `${platform}-${arch}`;
  return { triple, short: triple, os: [platform], cpu: [arch] };
}

const cargoArgs = [
  "build",
  "--manifest-path",
  path.join(root, "Cargo.toml"),
  "-p",
  "spark-napi",
  "--features",
  "node",
];
if (release) cargoArgs.push("--release");

console.log(`cargo ${cargoArgs.join(" ")}`);
const build = spawnSync("cargo", cargoArgs, {
  cwd: root,
  stdio: "inherit",
  shell: false,
  windowsHide: true,
});
if (build.error) {
  console.error(build.error);
  process.exit(1);
}
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const targetDir = path.join(root, "target", profile);
const stem = "spark_napi";
const candidates = [];
if (process.platform === "win32") {
  candidates.push(`${stem}.dll`, `${stem}.node`);
} else if (process.platform === "darwin") {
  candidates.push(`lib${stem}.dylib`, `${stem}.dylib`, `${stem}.node`);
} else {
  candidates.push(`lib${stem}.so`, `${stem}.so`, `${stem}.node`);
}

let artifact = null;
for (const name of candidates) {
  const p = path.join(targetDir, name);
  if (existsSync(p)) {
    artifact = p;
    break;
  }
}

if (!artifact) {
  const deps = path.join(targetDir, "deps");
  if (existsSync(deps)) {
    for (const name of readdirSync(deps)) {
      const base = name.replace(/^lib/, "");
      if (
        (name === stem ||
          name.startsWith(`${stem}.`) ||
          base.startsWith(`${stem}.`) ||
          name.startsWith(`lib${stem}.`)) &&
        (name.endsWith(".dll") ||
          name.endsWith(".so") ||
          name.endsWith(".dylib") ||
          name.endsWith(".node"))
      ) {
        artifact = path.join(deps, name);
        break;
      }
    }
  }
}

if (!artifact) {
  console.error(`找不到 ${stem} 产物于 ${targetDir}`);
  process.exit(1);
}

const plat = platformInfo();
const outDir = path.join(
  root,
  "projects",
  "platforms",
  "native",
  `sparkle-engine-${plat.short}`,
);
mkdirSync(outDir, { recursive: true });

const binaryName = `sparkle-engine.${plat.triple}.node`;
const pkgName = `@game-gpt/sparkle-engine-${plat.short}`;
const repoDir = `projects/platforms/native/sparkle-engine-${plat.short}`;
const pkgJsonPath = path.join(outDir, "package.json");

if (!existsSync(pkgJsonPath)) {
  writeFileSync(
    pkgJsonPath,
    `${JSON.stringify(
      {
        name: pkgName,
        version: "0.1.0",
        private: true,
        description: `Prebuilt Node-API addon for \`@game-gpt/sparkle-engine\` (${plat.short}). Platform artifact only.`,
        license: "Apache-2.0",
        os: plat.os,
        cpu: plat.cpu,
        main: binaryName,
        files: [binaryName, "README.md"],
        engines: { node: ">=18" },
        preferUnplugged: true,
      },
      null,
      4,
    )}\n`,
  );
} else {
  try {
    const pkg = JSON.parse(readFileSync(pkgJsonPath, "utf8"));
    pkg.name = pkgName;
    pkg.private = true;
    pkg.os = plat.os;
    pkg.cpu = plat.cpu;
    pkg.main = binaryName;
    pkg.files = [binaryName, "README.md"];
    if (!pkg.license) pkg.license = "Apache-2.0";
    if (!pkg.engines) pkg.engines = { node: ">=18" };
    writeFileSync(pkgJsonPath, `${JSON.stringify(pkg, null, 4)}\n`);
  } catch {
    /* ignore */
  }
}

const readmePath = path.join(outDir, "README.md");
if (!existsSync(readmePath)) {
  writeFileSync(
    readmePath,
    `# \`${pkgName}\`\n\n` +
      `原生 N-API 平台袋。\`main\` 为 \`${binaryName}\`（由 \`node scripts/build/napi.mjs\` 写入）。\n` +
      `请安装元包 \`@game-gpt/sparkle-engine\`，由包管理器选择本 optionalDependency。\n`,
  );
}

const destNamed = path.join(outDir, binaryName);
copyFileSync(artifact, destNamed);

// 清理旧名产物
for (const stale of [
  "spark.node",
  "sparkle-engine.node",
  `spark.${plat.triple}.node`,
]) {
  const p = path.join(outDir, stale);
  if (existsSync(p) && p !== destNamed) {
    try {
      unlinkSync(p);
    } catch {
      /* ignore */
    }
  }
}

console.log(`napi: ${artifact} → ${destNamed}`);
console.log(`Platform package: ${pkgName} (${outDir})`);
