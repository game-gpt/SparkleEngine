import { createRequire } from "node:module";

import {
  detectNativePlatformPackage,
  listPlatformPackages,
  resolveNativePath,
  type SparkPlatformPackage,
} from "./platform";

export {
  detectNativePlatformPackage,
  listPlatformPackages,
  platformShort,
  platformTriple,
  resolveNativePath,
  type PlatformInfo,
  type SparkNativeShort,
  type SparkPlatformPackage,
} from "./platform";

const nodeRequire = createRequire(__filename);

/** 与 `spark-napi`（`JsSparkHost`）对齐的最小 JS API。 */
export interface SparkHostBindings {
  info(): { name: string; version: string; npmPackage: string };
  vec2Length(x: number, y: number): number;
  loadBytes(root: string, key: string): number;
  assetLen(id: number): number | null | undefined;
}

/** napi-rs 导出的构造器（类名 `JsSparkHost`）。 */
interface SparkAddon {
  JsSparkHost: new () => {
    info(): { name: string; version: string; npmPackage: string };
    vec2Length(x: number, y: number): number;
    loadBytes(root: string, key: string): number;
    assetLen(id: number): number | null | undefined;
  };
}

export interface LoadOptions {
  /**
   * 强制指定平台包名（文档 / 探测用）。
   * 原生加载始终走 `resolveNativePath()`（或 `SPARK_NATIVE_NODE`）；
   * Wasm 请直接使用 `@game-gpt/sparkle-engine-unknown-wasm32` 的入口，不要经本函数。
   */
  platformPackage?: SparkPlatformPackage;
}

let _cached: SparkHostBindings | undefined;

/**
 * 加载当前平台的 Spark 原生绑定。
 * 平台包是**仅含 `.node` 的二进制袋**（`main` = `sparkle-engine.<triple>.node`），不是 TypeScript 包。
 */
export function loadSpark(_options: LoadOptions = {}): SparkHostBindings {
  if (_cached) return _cached;
  if (_options.platformPackage === "@game-gpt/sparkle-engine-unknown-wasm32") {
    throw new Error(
      "`loadSpark` 只加载原生 `.node`。Wasm 请使用 `@game-gpt/sparkle-engine-unknown-wasm32` 包自己的入口。",
    );
  }
  const addonPath = resolveNativePath();
  const addon = nodeRequire(addonPath) as SparkAddon;
  if (typeof addon.JsSparkHost !== "function") {
    throw new Error(
      `原生插件缺少 JsSparkHost：${addonPath}。请重新运行 node scripts/build/napi.mjs`,
    );
  }
  const host = new addon.JsSparkHost();
  _cached = {
    info: () => host.info(),
    vec2Length: (x, y) => host.vec2Length(x, y),
    loadBytes: (root, key) => host.loadBytes(root, key),
    assetLen: (id) => host.assetLen(id),
  };
  return _cached;
}

/** 当前探测到的原生平台包名（不加载二进制）。 */
export function currentNativePackage() {
  return detectNativePlatformPackage();
}
