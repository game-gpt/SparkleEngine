//! Steam 后端接口（与 steamworks / 其它实现解耦）。

use spark_core::SparkError;

/// 可替换的 Steam 平台后端。
pub trait SteamBackend: Send {
    fn is_available(&self) -> bool;
    fn app_id(&self) -> u32;
    fn user_name(&self) -> String;

    fn unlock_achievement(&mut self, id: &str) -> Result<(), SparkError>;
    fn is_achievement_unlocked(&self, id: &str) -> Result<bool, SparkError>;
    fn clear_achievement(&mut self, id: &str) -> Result<(), SparkError>;

    fn get_stat(&self, name: &str) -> Result<f32, SparkError>;
    fn set_stat(&mut self, name: &str, value: f32) -> Result<(), SparkError>;
    /// 将成就 / 统计刷到平台（占位后端为 no-op 成功）。
    fn store_stats(&mut self) -> Result<(), SparkError>;

    fn cloud_read(&self, path: &str) -> Result<Option<String>, SparkError>;
    fn cloud_write(&mut self, path: &str, data: &str) -> Result<(), SparkError>;
    fn cloud_delete(&mut self, path: &str) -> Result<bool, SparkError>;

    /// 打开叠加页 URL（网页 / 商店等）；默认可空实现。
    fn overlay_open_url(&mut self, url: &str) -> Result<(), SparkError> {
        let _ = url;
        Ok(())
    }
}

/// 无 Steamworks 时的占位后端：内存成就 / 统计 / 云文件。
#[derive(Debug, Clone)]
pub struct NullSteamBackend {
    pub app_id: u32,
    pub user_name: String,
    achievements: std::collections::HashMap<String, bool>,
    stats: std::collections::HashMap<String, f32>,
    cloud: std::collections::HashMap<String, String>,
}

impl Default for NullSteamBackend {
    fn default() -> Self {
        Self {
            app_id: 0,
            user_name: "offline".into(),
            achievements: std::collections::HashMap::new(),
            stats: std::collections::HashMap::new(),
            cloud: std::collections::HashMap::new(),
        }
    }
}

impl NullSteamBackend {
    pub fn new(app_id: u32, user_name: impl Into<String>) -> Self {
        Self { app_id, user_name: user_name.into(), ..Self::default() }
    }
}

impl SteamBackend for NullSteamBackend {
    fn is_available(&self) -> bool {
        false
    }

    fn app_id(&self) -> u32 {
        self.app_id
    }

    fn user_name(&self) -> String {
        self.user_name.clone()
    }

    fn unlock_achievement(&mut self, id: &str) -> Result<(), SparkError> {
        self.achievements.insert(id.into(), true);
        tracing::info!(event = "spark.steam.achievement_unlocked", achievement = id);
        Ok(())
    }

    fn is_achievement_unlocked(&self, id: &str) -> Result<bool, SparkError> {
        Ok(self.achievements.get(id).copied().unwrap_or(false))
    }

    fn clear_achievement(&mut self, id: &str) -> Result<(), SparkError> {
        self.achievements.insert(id.into(), false);
        Ok(())
    }

    fn get_stat(&self, name: &str) -> Result<f32, SparkError> {
        Ok(self.stats.get(name).copied().unwrap_or(0.0))
    }

    fn set_stat(&mut self, name: &str, value: f32) -> Result<(), SparkError> {
        self.stats.insert(name.into(), value);
        Ok(())
    }

    fn store_stats(&mut self) -> Result<(), SparkError> {
        Ok(())
    }

    fn cloud_read(&self, path: &str) -> Result<Option<String>, SparkError> {
        Ok(self.cloud.get(path).cloned())
    }

    fn cloud_write(&mut self, path: &str, data: &str) -> Result<(), SparkError> {
        self.cloud.insert(path.into(), data.into());
        Ok(())
    }

    fn cloud_delete(&mut self, path: &str) -> Result<bool, SparkError> {
        Ok(self.cloud.remove(path).is_some())
    }

    fn overlay_open_url(&mut self, url: &str) -> Result<(), SparkError> {
        tracing::debug!(event = "spark.steam.overlay_open_url", url);
        Ok(())
    }
}
