/**
 * 将 `spark-wasm` 的 release Wasm 拷到本包根目录 `spark_engine_bg.wasm`。
 *
 * 用法（在仓库 SparkEngine 根）：
 *   node projects/platforms/wasm/sparkle-engine-unknown-wasm32/scripts/copy-from-cargo.mjs
 */
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkgRoot = resolve(here, "..");
// projects/platforms/wasm/<pkg> → 仓库根
const engineRoot = resolve(pkgRoot, "../../../..");
const src = join(
  engineRoot,
  "target/wasm32-unknown-unknown/release/spark_wasm.wasm",
);
const dest = join(pkgRoot, "spark_engine_bg.wasm");

if (!existsSync(src)) {
  console.error(
    `未找到 ${src}\n请先：rustup target add wasm32-unknown-unknown\n然后：cargo build -p spark-wasm --target wasm32-unknown-unknown --release`,
  );
  process.exit(1);
}

mkdirSync(pkgRoot, { recursive: true });
copyFileSync(src, dest);
console.log(`已拷贝 → ${dest}`);
