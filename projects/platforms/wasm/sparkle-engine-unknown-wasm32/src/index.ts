/**
 * `spark-unknown-wasm32`：Wasm 目标的 TypeScript 加载器。
 *
 * 产物来自 Cargo 包 `spark-wasm`：
 * `target/wasm32-unknown-unknown/release/spark_wasm.wasm`
 * → 本包根目录 `spark_engine_bg.wasm`（见 `scripts/copy-from-cargo.mjs`）。
 */

export const rustTarget = "wasm32-unknown-unknown" as const;
export const platformPackage = "spark-unknown-wasm32" as const;

export interface WasmEngineInfo {
    name: string;
    version: string;
    npm_package: string;
    versionCode: number;
}

export interface SparkWasmBindings {
    info(): WasmEngineInfo;
    vec2Length(x: number, y: number): number;
}

export interface LoadWasmOptions {
    /** 自定义 wasm URL；默认相对本包的 `spark_engine_bg.wasm`。 */
    wasmUrl?: string | URL;
    /** 可选：已实例化的 WebAssembly.Module。 */
    module?: WebAssembly.Module;
}

type WasmExports = {
    spark_vec2_length: (x: number, y: number) => number;
    spark_version_code: () => number;
    memory?: WebAssembly.Memory;
};

function decodeVersion(code: number): string {
    const major = Math.floor(code / 1_000_000);
    const minor = Math.floor((code % 1_000_000) / 1_000);
    const patch = code % 1_000;
    return `${major}.${minor}.${patch}`;
}

/**
 * 加载 `spark-wasm` 产出的模块。
 * 若尚未拷贝 `.wasm`，回退纯 JS 几何实现以便 TS 联调。
 */
export async function loadSpark(options: LoadWasmOptions = {}): Promise<SparkWasmBindings> {
    const url = options.wasmUrl ?? new URL("../spark_engine_bg.wasm", import.meta.url);

    let exports: Partial<WasmExports> | undefined;
    try {
        if (options.module) {
            const instance = await WebAssembly.instantiate(options.module, {});
            exports = instance.exports as unknown as WasmExports;
        } else if (typeof fetch === "function") {
            const result = await WebAssembly.instantiateStreaming(fetch(url.toString()), {});
            exports = result.instance.exports as unknown as WasmExports;
        } else {
            const { readFile } = await import("node:fs/promises");
            const { fileURLToPath } = await import("node:url");
            const buf = await readFile(fileURLToPath(url));
            const result = await WebAssembly.instantiate(buf, {});
            exports = result.instance.exports as unknown as WasmExports;
        }
    } catch {
        exports = undefined;
    }

    if (exports?.spark_vec2_length && exports.spark_version_code) {
        const vec2Length = exports.spark_vec2_length;
        const versionCode = exports.spark_version_code();
        return {
            info: () => ({
                name: "Spark Engine",
                version: decodeVersion(versionCode),
                npm_package: platformPackage,
                versionCode,
            }),
            vec2Length: (x, y) => vec2Length(x, y),
        };
    }

    return {
        info: () => ({
            name: "Spark Engine",
            version: "0.0.0",
            npm_package: platformPackage,
            versionCode: 0,
        }),
        vec2Length: (x, y) => Math.hypot(x, y),
    };
}

export default { loadSpark, rustTarget, platformPackage };
