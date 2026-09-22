//! 命名钩子总线：模组脚本注册，宿主按时机触发。

/// 已注册钩子回调的定位：哪个模组的哪个导出函数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRef {
    /// 所属模组 id（与清单 `id` 一致）。
    pub mod_id: String,
    /// 脚本导出函数名（由 VM 按名调用）。
    pub function: String,
}

/// 钩子名 → 回调列表。同名按注册顺序触发；热重载时按模组清条目。
#[derive(Debug, Default)]
pub struct HookBus {
    /// hook 名 → 回调列表
    map: std::collections::HashMap<String, Vec<HookRef>>,
}

impl HookBus {
    /// 在 `hook` 名下追加一条回调（不查重；同模组可注册多次）。
    pub fn register(&mut self, hook: impl Into<String>, mod_id: impl Into<String>, function: impl Into<String>) {
        self.map.entry(hook.into()).or_default().push(HookRef { mod_id: mod_id.into(), function: function.into() });
    }

    /// 返回某钩子名下的全部回调；未注册时为空切片。
    pub fn list(&self, hook: &str) -> &[HookRef] {
        self.map.get(hook).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 移除指定模组的全部回调（热重载 / 卸载前调用）。
    pub fn remove_mod(&mut self, mod_id: &str) {
        for list in self.map.values_mut() {
            list.retain(|h| h.mod_id != mod_id);
        }
    }

    /// 清空全部钩子登记。
    pub fn clear(&mut self) {
        self.map.clear();
    }
}
