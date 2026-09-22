//! 可被 JS / 测试共用的宿主门面。

use spark_asset::{AssetCache, AssetKey, BytesLoader};
use spark_edit::{EditCapabilities, EditMode, run_edit_plan_with};
use spark_types::Vec2;

/// 引擎身份信息（名称 / 版本 / npm 包名），供 JS 侧展示与探针。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInfo {
    /// 产品显示名。
    pub name: &'static str,
    /// crate 版本（`CARGO_PKG_VERSION`）。
    pub version: &'static str,
    /// npm 包名（与 [`crate::NPM_PACKAGE_NAME`] 一致）。
    pub npm_package: &'static str,
}
impl Default for EngineInfo {
    fn default() -> Self {
        Self { name: "Spark Engine", version: env!("CARGO_PKG_VERSION"), npm_package: crate::NPM_PACKAGE_NAME }
    }
}

/// JS 友好的同步 API 子集（无窗口 / 无 ECS 世界权威）。
#[derive(Debug, Default)]
pub struct SparkJsHost {
    /// 进程内资产字节缓存。
    pub assets: AssetCache,
}

impl SparkJsHost {
    /// 空宿主（默认资产缓存）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回当前引擎身份信息。
    pub fn info(&self) -> EngineInfo {
        EngineInfo::default()
    }

    /// 向量长度（演示几何桥接）。
    pub fn vec2_length(&self, x: f64, y: f64) -> f64 {
        Vec2::new(x as f32, y as f32).length() as f64
    }

    /// 从目录加载字节资源，返回缓存 id。
    pub fn load_bytes(&mut self, root: &str, key: &str) -> Result<u32, spark_asset::AssetError> {
        let loader = BytesLoader::new(root);
        let id = self.assets.load(AssetKey::new(key), &loader)?;
        Ok(id.0)
    }

    /// 已缓存资产的字节长度；未知 id 返回 `None`。
    pub fn asset_len(&self, id: u32) -> Option<usize> {
        self.assets.bytes(spark_asset::AssetId(id)).map(|b| b.len())
    }

    /// 运行 VON 编辑计划，返回 JSON 报告（供 MCP / JS Agent）。
    ///
    /// `mode`：`check` | `dry-run` | `apply`。
    /// `capabilities`：空则 check/dry-run 只读、apply 用 project-edit；非空则按 token 解析。
    pub fn run_edit_plan(&self, root: &str, mode: &str, von: &str) -> Result<String, String> {
        self.run_edit_plan_with_caps(root, mode, von, &[])
    }

    /// 带显式能力 token 的编辑入口。
    pub fn run_edit_plan_with_caps(
        &self,
        root: &str,
        mode: &str,
        von: &str,
        capabilities: &[String],
    ) -> Result<String, String> {
        let mode = EditMode::parse(mode).ok_or_else(|| "spark.edit.bad_mode".to_string())?;
        let caps = if capabilities.is_empty() {
            match mode {
                EditMode::Apply => EditCapabilities::project_edit(),
                _ => EditCapabilities::read_only(),
            }
        } else {
            EditCapabilities::from_tokens(capabilities)
        };
        let report = run_edit_plan_with(root, mode, von, caps)?;
        Ok(report.to_json())
    }
}
