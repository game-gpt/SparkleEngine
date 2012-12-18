//! 命名钩子总线：模组脚本注册，宿主按时机触发。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRef {
    pub mod_id: String,
    pub function: String,
}

#[derive(Debug, Default)]
pub struct HookBus {
    /// hook 名 → 回调列表
    map: std::collections::HashMap<String, Vec<HookRef>>,
}

impl HookBus {
    pub fn register(&mut self, hook: impl Into<String>, mod_id: impl Into<String>, function: impl Into<String>) {
        self.map.entry(hook.into()).or_default().push(HookRef { mod_id: mod_id.into(), function: function.into() });
    }

    pub fn list(&self, hook: &str) -> &[HookRef] {
        self.map.get(hook).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn remove_mod(&mut self, mod_id: &str) {
        for list in self.map.values_mut() {
            list.retain(|h| h.mod_id != mod_id);
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}
