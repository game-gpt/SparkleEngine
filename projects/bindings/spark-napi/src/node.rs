//! napi 导出（feature = `node`）。

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::host::SparkJsHost;

#[napi(object)]
pub struct JsEngineInfo {
    pub name: String,
    pub version: String,
    pub npm_package: String,
}

#[napi]
pub struct JsSparkHost {
    inner: SparkJsHost,
}

#[napi]
impl JsSparkHost {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { inner: SparkJsHost::new() }
    }

    #[napi]
    pub fn info(&self) -> JsEngineInfo {
        let i = self.inner.info();
        JsEngineInfo { name: i.name.into(), version: i.version.into(), npm_package: i.npm_package.into() }
    }

    #[napi]
    pub fn vec2_length(&self, x: f64, y: f64) -> f64 {
        self.inner.vec2_length(x, y)
    }

    #[napi]
    pub fn load_bytes(&mut self, root: String, key: String) -> Result<u32> {
        // FFI 边界：向 JS 暴露稳定错误码（Display），不是自然语言句子。
        self.inner.load_bytes(&root, &key).map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn asset_len(&self, id: u32) -> Option<u32> {
        self.inner.asset_len(id).map(|n| n as u32)
    }

    /// 执行 VON 编辑计划，返回 JSON 字符串报告。
    #[napi]
    pub fn run_edit_plan(&self, root: String, mode: String, von: String) -> Result<String> {
        self.inner
            .run_edit_plan(&root, &mode, &von)
            .map_err(|e| Error::from_reason(e))
    }

    /// 带能力 token 的编辑入口（如 `project-edit`）。
    #[napi]
    pub fn run_edit_plan_with_caps(
        &self,
        root: String,
        mode: String,
        von: String,
        capabilities: Vec<String>,
    ) -> Result<String> {
        self.inner
            .run_edit_plan_with_caps(&root, &mode, &von, &capabilities)
            .map_err(|e| Error::from_reason(e))
    }
}
