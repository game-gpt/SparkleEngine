import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";

const nodeRequire = createRequire(__filename);

/** 原生平台包短名（npm：`spark-<short>`）。 */
export type SparkNativeShort =
  | "win32-x64"
  | "win32-arm64"
  | "darwin-x64"
  | "darwin-arm64"
  | "linux-x64"
  | "linux-arm64";

export type SparkPlatformPackage =
  | `spark-${SparkNativeShort}`
  | "spark-unknown-wasm32";

export interface PlatformInfo {
  packageName: SparkPlatformPackage;
  /** napi-rs / 二进制文件名用的 triple（如 `win32-x64-msvc`）。 */
  triple: string;
  /** Rust target 提示（文档用）。 */
  rustTarget: string;
  kind: "native" | "wasm";
}

const TABLE: PlatformInfo[] = [
  {
    packageName: "spark-win32-x64",
    triple: "win32-x64-msvc",
    rustTarget: "x86_64-pc-windows-msvc",
    kind: "native",
  },
  {
    packageName: "spark-win32-arm64",
    triple: "win32-arm64-msvc",
    rustTarget: "aarch64-pc-windows-msvc",
    kind: "native",
  },
  {
    packageName: "spark-darwin-x64",
    triple: "darwin-x64",
    rustTarget: "x86_64-apple-darwin",
    kind: "native",
  },
  {
    packageName: "spark-darwin-arm64",
    triple: "darwin-arm64",
    rustTarget: "aarch64-apple-darwin",
    kind: "native",
  },
  {
    packageName: "spark-linux-x64",
    triple: "linux-x64-gnu",
    rustTarget: "x86_64-unknown-linux-gnu",
    kind: "native",
  },
  {
    packageName: "spark-linux-arm64",
    triple: "linux-arm64-gnu",
    rustTarget: "aarch64-unknown-linux-gnu",
    kind: "native",
  },
  {
    packageName: "spark-unknown-wasm32",
    triple: "wasm32-unknown-unknown",
    rustTarget: "wasm32-unknown-unknown",
    kind: "wasm",
  },
];

/** 全部已登记平台包。 */
export function listPlatformPackages(): readonly PlatformInfo[] {
  return TABLE;
}

/** 当前 Node 的 napi triple（如 `win32-x64-msvc`）。 */
export function platformTriple(
  platform: NodeJS.Platform = process.platform,
  arch: NodeJS.Architecture = process.arch,
): string {
  if (platform === "win32" && arch === "x64") return "win32-x64-msvc";
  if (platform === "win32" && arch === "arm64") return "win32-arm64-msvc";
  if (platform === "darwin" && arch === "x64") return "darwin-x64";
  if (platform === "darwin" && arch === "arm64") return "darwin-arm64";
  if (platform === "linux" && arch === "x64") return "linux-x64-gnu";
  if (platform === "linux" && arch === "arm64") return "linux-arm64-gnu";
  return `${platform}-${arch}`;
}

/** triple → 短名（npm 包后缀）。 */
export function platformShort(triple: string = platformTriple()): SparkNativeShort {
  if (triple === "win32-x64-msvc") return "win32-x64";
  if (triple === "win32-arm64-msvc") return "win32-arm64";
  if (triple === "linux-x64-gnu") return "linux-x64";
  if (triple === "linux-arm64-gnu") return "linux-arm64";
  if (triple === "darwin-x64" || triple === "darwin-arm64") {
    return triple as SparkNativeShort;
  }
  throw new Error(
    `不受支持的 Node 平台 triple：${triple}。可改用 spark-unknown-wasm32。`,
  );
}

/**
 * 根据 Node `process` 选择原生平台包名。
 * 非 Node 或显式 Wasm 场景请直接依赖 `spark-unknown-wasm32`。
 */
export function detectNativePlatformPackage(
  platform: NodeJS.Platform = process.platform,
  arch: NodeJS.Architecture = process.arch,
): `spark-${SparkNativeShort}` {
  return `spark-${platformShort(platformTriple(platform, arch))}`;
}

/**
 * 解析当前平台 `.node` 路径（optionalDependency + 仓库内 `packages/spark-<short>`）。
 * 对齐 vmz-framework：`main` 即二进制，经 `require.resolve` 定位包目录。
 */
export function resolveNativePath(): string {
  const envPath =
    (typeof process.env.SPARK_NATIVE_NODE === "string" &&
      process.env.SPARK_NATIVE_NODE.trim()) ||
    "";
  if (envPath) {
    const abs = path.resolve(envPath);
    if (!existsSync(abs)) {
      throw new Error(`SPARK_NATIVE_NODE 指向的文件不存在：${abs}`);
    }
    return abs;
  }

  const triple = platformTriple();
  const short = platformShort(triple);
  const name = `spark-${short}`;
  const binary = `spark.${triple}.node`;
  const candidates: string[] = [];

  try {
    const resolved = nodeRequire.resolve(`${name}/package.json`);
    const dir = path.dirname(resolved);
    candidates.push(path.join(dir, binary), path.join(dir, "spark.node"));
  } catch {
    /* optional dep 未安装 */
  }

  // 本包 node_modules（嵌套安装）
  candidates.push(
    path.join(__dirname, "..", "node_modules", name, binary),
    path.join(__dirname, "..", "node_modules", name, "spark.node"),
  );

  // 仓库布局：packages/spark-engine → packages/spark-<short>
  candidates.push(
    path.join(__dirname, "..", "..", name, binary),
    path.join(__dirname, "..", "..", name, "spark.node"),
  );

  for (const p of candidates) {
    if (existsSync(p)) return p;
  }

  throw new Error(
    `找不到原生插件 spark-${short}（${binary}）。` +
      `请运行：node scripts/build/napi.mjs\n` +
      `已查找：\n${candidates.map((c) => `  - ${c}`).join("\n")}`,
  );
}
