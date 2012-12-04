# `spark-win32-x64`

原生 N-API 平台包。`main` 为 `spark.win32-x64-msvc.node`（由 `node scripts/build/napi.mjs` 写入）。

元包 `spark-engine` 经 optionalDependencies + `require.resolve` 加载，**不是** TypeScript 包。
