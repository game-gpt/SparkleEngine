# `@game-gpt/sparkle-engine`

npm entry for Spark Engine: loads the native N-API plugin (`.node`) for the current platform and provides the single-repo CLI `spark`.

## Install

```bash
npm install @game-gpt/sparkle-engine
# or pnpm add @game-gpt/sparkle-engine
```

Node.js ≥ 18. Matching optionalDependencies (e.g. `@game-gpt/sparkle-engine-win32-x64`) install with the host; if no platform matches, build the native bundle yourself.

Development in this repo:

```bash
pnpm install
pnpm run build:ts
pnpm run build:napi    # writes to projects/platforms/native/sparkle-engine-<short>/
```

## JavaScript API

```js
const { loadSpark, currentNativePackage, listPlatformPackages } = require("@game-gpt/sparkle-engine");

console.log(currentNativePackage());
// → "@game-gpt/sparkle-engine-win32-x64", etc.

const spark = loadSpark();
console.log(spark.info());
// → { name, version, npmPackage }

console.log(spark.vec2Length(3, 4)); // 5

const id = spark.loadBytes("/path/to/assets", "a.txt");
console.log(spark.assetLen(id));
```

- `loadSpark()`: `require` the current platform `.node`, construct `JsSparkHost`, cache singleton.
- Force Wasm: do not use `loadSpark`; use `loadSpark` from `@game-gpt/sparkle-engine-unknown-wasm32`.
- Override binary path: environment variable `SPARK_NATIVE_NODE` pointing at a `.node` file.

## CLI `spark`

```bash
pnpm exec spark info
pnpm exec spark run [--cwd <game-project>]
pnpm exec spark studio [--cwd <game-project>] [--play] [--safe-mode]
```

| Command | Behavior |
|---------|----------|
| `info` | Load native binding and print `info()` JSON |
| `run` | Read project `package.json` `spark.runTarget` → `cargo run -p …`; otherwise `spark-studio --play` |
| `studio` | Launch `spark-studio` binary (requires `cargo build -p spark-studio` first) |

Game project directories must contain `package.json`. Studio binary can also be set via `SPARK_STUDIO_BIN`.

## Platform packages

| Package | Purpose |
|---------|---------|
| `@game-gpt/sparkle-engine-win32-x64`, etc. | Prebuilt `.node` only |
| `@game-gpt/sparkle-engine-unknown-wasm32` | Browser / Wasm loader + `.wasm` |

See `listPlatformPackages()`.

## License

Apache-2.0
