//! 游戏可替换的脚本宿主 API 装配（[`ScriptApiProvider`]）。

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use spark_script::{HostEffect, HostFunction, HostFunctionId, HostPhase, HostSchema};
use spark_vm::Vm;

use crate::EngineShared;

/// 向宿主 schema 注册可选导入的游戏侧 API 面。
pub trait ScriptApiProvider: Send + Sync {
    /// 向 `schema` 追加本 Provider 提供的宿主导入声明。
    fn register(&self, schema: &mut HostSchema);

    /// 模组装载后向 VM 安装本 Provider 对应的原生实现（默认无操作）。
    fn install_vm(&self, _vm: &mut Vm, _shared: &Rc<RefCell<EngineShared>>) {}
}

/// 引擎内置 Provider（占位；核心 `engine.*` 由 [`crate::api::engine_host_schema`] 登记）。
#[derive(Debug, Clone, Copy, Default)]
pub struct CoreEngineScriptApiProvider;

impl ScriptApiProvider for CoreEngineScriptApiProvider {
    fn register(&self, _schema: &mut HostSchema) {}
}

/// 示例游戏战斗 API Provider（演示 `game.*` 面扩展；运行时尚未接线原生实现）。
#[derive(Debug, Clone, Copy, Default)]
pub struct GameCombatScriptApiProvider;

impl ScriptApiProvider for GameCombatScriptApiProvider {
    fn register(&self, schema: &mut HostSchema) {
        let sim = [HostPhase::OnLoad, HostPhase::OnStart, HostPhase::FixedUpdate, HostPhase::Update, HostPhase::LateUpdate, HostPhase::OnEvent];
        schema.insert(
            HostFunction::new(HostFunctionId::new("game", "apply_damage", 2))
                .phases(sim)
                .effect(HostEffect::WriteComponent),
        );
        schema.insert(HostFunction::new(HostFunctionId::new("game", "is_alive", 1)).phases(sim).effect(HostEffect::ReadWorld));
    }

    fn install_vm(&self, vm: &mut Vm, shared: &Rc<RefCell<EngineShared>>) {
        crate::game_api::install_game_combat_natives(vm, shared);
    }
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

    /// 向模组 VM 安装已登记 Provider 的原生实现。
    pub fn install_vm_natives(&self, vm: &mut Vm, shared: &Rc<RefCell<EngineShared>>) {
        for provider in &self.providers {
            provider.install_vm(vm, shared);
        }
    }
}
