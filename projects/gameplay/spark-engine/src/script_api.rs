//! 游戏可替换的脚本宿主 API 装配（[`ScriptApiProvider`]）。

use std::collections::HashSet;

use spark_script::HostSchema;

/// 向宿主 schema 注册可选导入的游戏侧 API 面。
pub trait ScriptApiProvider: Send + Sync {
    /// 向 `schema` 追加本 Provider 提供的宿主导入声明。
    fn register(&self, schema: &mut HostSchema);
}

/// 引擎内置 Provider（占位；核心 `engine.*` 由 [`crate::api::engine_host_schema`] 登记）。
#[derive(Debug, Clone, Copy, Default)]
pub struct CoreEngineScriptApiProvider;

impl ScriptApiProvider for CoreEngineScriptApiProvider {
    fn register(&self, _schema: &mut HostSchema) {}
}

/// 多 Provider 装配表：可禁用单个限定名、可清空后只挂游戏 API。
#[derive(Default)]
pub struct ScriptApiRegistry {
    providers: Vec<Box<dyn ScriptApiProvider>>,
    disabled: HashSet<String>,
}

impl ScriptApiRegistry {
    /// 空登记表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加 Provider（在 `build_host_schema` 时按注册顺序应用）。
    pub fn register_provider(&mut self, provider: Box<dyn ScriptApiProvider>) {
        self.providers.push(provider);
    }

    /// 禁用限定导入名（如 `engine.queue_spawn` 或短名 `queue_spawn`）。
    pub fn disable(&mut self, qualified_name: impl Into<String>) {
        self.disabled.insert(qualified_name.into());
    }

    /// 清空全部 Provider（游戏可随后只挂自己的 API 面）。
    pub fn clear_providers(&mut self) {
        self.providers.clear();
    }

    /// 将已登记 Provider 写入 schema，并剔除 `disabled` 中的函数。
    pub fn apply_to(&self, schema: &mut HostSchema) {
        for provider in &self.providers {
            provider.register(schema);
        }
        if self.disabled.is_empty() {
            return;
        }
        schema.functions.retain(|func| {
            let qualified = func.id.qualified_name();
            let short = func.id.name.as_ref();
            !self.disabled.contains(&qualified) && !self.disabled.contains(short)
        });
    }
}
