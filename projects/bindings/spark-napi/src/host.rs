//! 可被 JS / 测试共用的宿主门面。

use spark_asset::{AssetCache, AssetKey, BytesLoader};
use spark_edit::{EditMode, run_edit_plan};
use spark_types::Vec2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineInfo {    pub name: &'static str,
    pub version: &'static str,
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
    pub assets: AssetCache,
}

impl SparkJsHost {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn asset_len(&self, id: u32) -> Option<usize> {
        self.assets.bytes(spark_asset::AssetId(id)).map(|b| b.len())
    }

    /// 运行 VON 编辑计划，返回 JSON 报告（供 MCP / JS Agent）。
    ///
    /// `mode`：`check` | `dry-run` | `apply`。解析失败返回 `Err`。
    pub fn run_edit_plan(&self, root: &str, mode: &str, von: &str) -> Result<String, String> {
        let mode = EditMode::parse(mode).ok_or_else(|| "spark.edit.bad_mode".to_string())?;
        let report = run_edit_plan(root, mode, von)?;
        Ok(report.to_json())
    }
}
