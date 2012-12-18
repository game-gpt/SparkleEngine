/**
 * `spark-unknown-wasm32`：Wasm 目标的 TypeScript 加载器。
 *
 * 发布前放入同级 `spark_engine_bg.wasm`（由后续 `wasm-bindgen` / 专用 crate 产出）。
 * 本包不依赖 Node 原生插件。
 */
export declare const rustTarget: "wasm32-unknown-unknown";
export declare const platformPackage: "spark-unknown-wasm32";
export interface WasmEngineInfo {
    name: string;
    version: string;
    npm_package: string;
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
/**
 * 加载 Wasm 模块并返回精简绑定。
 * 若尚未放入真实 `.wasm`，仍返回可用的 JS 回退实现（几何演示），便于 TS 联调。
 */
export declare function loadSpark(options?: LoadWasmOptions): Promise<SparkWasmBindings>;
declare const _default: {
    loadSpark: typeof loadSpark;
    rustTarget: "wasm32-unknown-unknown";
    platformPackage: "spark-unknown-wasm32";
};
export default _default;
//# sourceMappingURL=index.d.ts.map
