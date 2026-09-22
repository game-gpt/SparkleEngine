//! Steam 后端接口（与 steamworks / 其它实现解耦）。

use spark_types::SparkError;

/// 可替换的 Steam 平台后端：成就、统计、云存储与叠加层。
///
/// 方法返回 [`SparkError`] 表示平台或 IO 失败；脚本侧会映射为
/// [`spark_vm::VmError::BadNativeArg`]（`name = "backend"`）。
pub trait SteamBackend: Send {
    /// 客户端是否已就绪（占位后端恒为 `false`）。
    fn is_available(&self) -> bool;
    /// Steam AppID；占位后端可为 `0`。
    fn app_id(&self) -> u32;
    /// 当前用户显示名。
    fn user_name(&self) -> String;

    /// 解锁成就 `id`（平台侧 API 名）。
    fn unlock_achievement(&mut self, id: &str) -> Result<(), SparkError>;
    /// 查询成就是否已解锁；未知 id 视为未解锁。
    fn is_achievement_unlocked(&self, id: &str) -> Result<bool, SparkError>;
    /// 清除成就解锁状态（调试/测试用）。
    fn clear_achievement(&mut self, id: &str) -> Result<(), SparkError>;

    /// 读取浮点统计；未知名返回 `0.0`。
    fn get_stat(&self, name: &str) -> Result<f32, SparkError>;
    /// 写入浮点统计（本地缓存；需再调 [`SteamBackend::store_stats`] 刷盘）。
    fn set_stat(&mut self, name: &str, value: f32) -> Result<(), SparkError>;
    /// 将成就 / 统计刷到平台（占位后端为 no-op 成功）。
    fn store_stats(&mut self) -> Result<(), SparkError>;

    /// 读云文件；不存在返回 `Ok(None)`。
    fn cloud_read(&self, path: &str) -> Result<Option<String>, SparkError>;
    /// 写云文件（整文件覆盖）。
    fn cloud_write(&mut self, path: &str, data: &str) -> Result<(), SparkError>;
    /// 删云文件；返回是否原先存在。
    fn cloud_delete(&mut self, path: &str) -> Result<bool, SparkError>;

    /// 打开叠加页 URL（网页 / 商店等）；默认可空实现。
    fn overlay_open_url(&mut self, url: &str) -> Result<(), SparkError> {
        let _ = url;
        Ok(())
    }
}

/// 无 Steamworks 时的占位后端：内存成就 / 统计 / 云文件。
///
/// [`is_available`](SteamBackend::is_available) 恒为 `false`；数据仅进程内有效。
#[derive(Debug, Clone)]
pub struct NullSteamBackend {
    /// 对外报告的 AppID（默认 `0`）。
    pub app_id: u32,
    /// 对外报告的用户名（默认 `"offline"`）。
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
    /// 指定 AppID 与用户名的空内存后端。
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
