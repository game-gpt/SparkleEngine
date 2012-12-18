/**
 * `spark-unknown-wasm32`：Wasm 目标的 TypeScript 加载器。
 *
 * 发布前放入同级 `spark_engine_bg.wasm`（由后续 `wasm-bindgen` / 专用 crate 产出）。
 * 本包不依赖 Node 原生插件。
 */
export const rustTarget = "wasm32-unknown-unknown";
export const platformPackage = "spark-unknown-wasm32";
/**
 * 加载 Wasm 模块并返回精简绑定。
 * 若尚未放入真实 `.wasm`，仍返回可用的 JS 回退实现（几何演示），便于 TS 联调。
 */
export async function loadSpark(options = {}) {
    const url = options.wasmUrl ?? new URL("../spark_engine_bg.wasm", import.meta.url);
    let exports;
    try {
        if (options.module) {
            const instance = await WebAssembly.instantiate(options.module, {});
            exports = instance.exports;
        } else {
            const result = await WebAssembly.instantiateStreaming(fetch(url.toString()), {});
            exports = result.instance.exports;
        }
    } catch {
        exports = undefined;
    }
    if (exports?.spark_vec2_length) {
        const vec2Length = exports.spark_vec2_length;
        return {
            info: () => ({
                name: "Spark Engine",
                version: "0.1.0",
                npm_package: "spark-unknown-wasm32",
            }),
            vec2Length: (x, y) => vec2Length(x, y),
        };
    }
    // 无 wasm 产物时的纯 JS 回退（不替代正式引擎）。
    return {
        info: () => ({
            name: "Spark Engine",
            version: "0.1.0",
            npm_package: "spark-unknown-wasm32",
        }),
        vec2Length: (x, y) => Math.hypot(x, y),
    };
}
export default { loadSpark, rustTarget, platformPackage };
//# sourceMappingURL=index.js.map
