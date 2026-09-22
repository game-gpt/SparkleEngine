//! Live2D 后端接口（与具体 SDK 解耦）。

use spark_types::{ErrorArg, ErrorCode, SparkError};

use crate::runtime::Live2dModelId;

fn model_invalid() -> ErrorCode {
    ErrorCode::new("spark.plugin.live2d", "model_invalid")
}

/// Cubism / 其它运行时的可替换后端。
pub trait Live2dBackend: Send {
    fn load(&mut self, path: &str) -> Result<Live2dModelId, SparkError>;
    fn unload(&mut self, id: Live2dModelId) -> Result<(), SparkError>;
    fn set_param(&mut self, id: Live2dModelId, name: &str, value: f32) -> Result<(), SparkError>;
    fn get_param(&self, id: Live2dModelId, name: &str) -> Result<f32, SparkError>;
    fn update(&mut self, id: Live2dModelId, dt: f32) -> Result<(), SparkError>;
    /// 播放动作（名称由资源约定）；默认可空实现。
    fn start_motion(&mut self, id: Live2dModelId, group: &str, index: i32) -> Result<(), SparkError> {
        let _ = (id, group, index);
        Ok(())
    }
}

/// 无 SDK 时的占位后端：只存参数表，便于脚本联调。
#[derive(Debug, Default)]
pub struct NullLive2dBackend {
    next: u32,
    models: std::collections::HashMap<u32, ModelStub>,
}

#[derive(Debug, Default)]
struct ModelStub {
    #[allow(dead_code)]
    path: String,
    params: std::collections::HashMap<String, f32>,
}

impl Live2dBackend for NullLive2dBackend {
    fn load(&mut self, path: &str) -> Result<Live2dModelId, SparkError> {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        self.models.insert(id, ModelStub { path: path.into(), params: std::collections::HashMap::new() });
        tracing::info!(event = "spark.live2d.model_loaded", path, model = id);
        Ok(Live2dModelId(id))
    }

    fn unload(&mut self, id: Live2dModelId) -> Result<(), SparkError> {
        if self.models.remove(&id.0).is_none() {
            return Err(invalid_model(id));
        }
        Ok(())
    }

    fn set_param(&mut self, id: Live2dModelId, name: &str, value: f32) -> Result<(), SparkError> {
        let m = self.models.get_mut(&id.0).ok_or_else(|| invalid_model(id))?;
        m.params.insert(name.into(), value);
        Ok(())
    }

    fn get_param(&self, id: Live2dModelId, name: &str) -> Result<f32, SparkError> {
        let m = self.models.get(&id.0).ok_or_else(|| invalid_model(id))?;
        Ok(m.params.get(name).copied().unwrap_or(0.0))
    }

    fn update(&mut self, id: Live2dModelId, dt: f32) -> Result<(), SparkError> {
        let _ = dt;
        if !self.models.contains_key(&id.0) {
            return Err(invalid_model(id));
        }
        Ok(())
    }

    fn start_motion(&mut self, id: Live2dModelId, group: &str, index: i32) -> Result<(), SparkError> {
        if !self.models.contains_key(&id.0) {
            return Err(invalid_model(id));
        }
        tracing::debug!(event = "spark.live2d.start_motion", ?id, group, index);
        Ok(())
    }
}

fn invalid_model(id: Live2dModelId) -> SparkError {
    SparkError::new(model_invalid()).arg("id", ErrorArg::Unsigned(id.0 as u64))
}
