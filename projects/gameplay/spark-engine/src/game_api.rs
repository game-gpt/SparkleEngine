//! 示例 `game.*` 宿主导入实现（与 [`GameCombatScriptApiProvider`] 配对）。

use std::{cell::RefCell, rc::Rc};

use spark_gc::Value;
use spark_vm::{Vm, VmError};

use crate::{
    EngineShared,
    access_policy::{check_host_determinism, check_host_phase},
};

/// 战斗示例使用的生命值组件名（游戏仓可替换为自有 Provider + 原生）。
pub const GAME_HEALTH_COMPONENT: &str = "Health";

/// 安装 `game.apply_damage` / `game.is_alive` 原生（由 [`GameCombatScriptApiProvider`] 触发）。
pub fn install_game_combat_natives(vm: &mut Vm, shared: &Rc<RefCell<EngineShared>>) {
    let shared_dmg = Rc::clone(shared);
    vm.register_native("game.apply_damage", move |_ctx, args| {
        let import = "game.apply_damage";
        if args.len() < 2 {
            return Err(VmError::ArityMismatch { expected: 2, got: args.len() as u16 });
        }
        let entity = args[0].as_entity().ok_or(VmError::BadNativeArg { name: "entity" })?;
        let damage = args[1].as_number().ok_or(VmError::BadNativeArg { name: "damage" })? as f32;
        let mut shared = shared_dmg.borrow_mut();
        gate(&shared, import)?;
        gate_health_write(&shared)?;
        let (slot, layout) = health_layout_cloned(&shared)?;
        let store = shared.active_component_store.as_mut().ok_or(VmError::HostDenied {
            detail: "host_game_denied:no_component_store".into(),
        })?;
        let hp = store.read_f32(slot, &layout, entity, 0).unwrap_or(0.0);
        let next = (hp - damage).max(0.0);
        if !store.write_f32(slot, &layout, entity, 0, next) {
            return Err(VmError::BadNativeArg { name: import });
        }
        Ok(Value::Null)
    });

    let shared_alive = Rc::clone(shared);
    vm.register_native("game.is_alive", move |_ctx, args| {
        let import = "game.is_alive";
        let entity = args.first().and_then(|v| v.as_entity()).ok_or(VmError::BadNativeArg { name: "entity" })?;
        let shared = shared_alive.borrow();
        gate(&shared, import)?;
        gate_health_read(&shared)?;
        let (slot, layout) = health_layout_cloned(&shared)?;
        let store = shared.active_component_store.as_ref().ok_or(VmError::HostDenied {
            detail: "host_game_denied:no_component_store".into(),
        })?;
        let hp = store.read_f32(slot, &layout, entity, 0).unwrap_or(0.0);
        Ok(Value::Bool(hp > 0.0))
    });
}

fn gate(shared: &EngineShared, import: &str) -> Result<(), VmError> {
    if !shared.execution_profile.enforces_runtime_gate() {
        return Ok(());
    }
    check_host_phase(&shared.host_schema, import, shared.active_phase).map_err(|detail| VmError::HostDenied { detail })?;
    check_host_determinism(&shared.host_schema, import, shared.active_determinism).map_err(|detail| VmError::HostDenied { detail })?;
    Ok(())
}

fn health_layout_cloned(
    shared: &EngineShared,
) -> Result<(crate::command_apply::ComponentDescriptorId, crate::ScriptComponentLayout), VmError> {
    let slot = shared
        .active_component_catalog
        .id_of(GAME_HEALTH_COMPONENT)
        .ok_or(VmError::HostDenied { detail: "host_game_denied:unknown_health_component".into() })?;
    let layout = shared
        .active_component_catalog
        .layout_of(slot)
        .cloned()
        .ok_or(VmError::HostDenied { detail: "host_game_denied:no_health_layout".into() })?;
    Ok((slot, layout))
}

fn gate_health_write(shared: &EngineShared) -> Result<(), VmError> {
    if !shared.execution_profile.enforces_runtime_gate() {
        return Ok(());
    }
    if shared.access.allows_write(GAME_HEALTH_COMPONENT) {
        Ok(())
    }
    else {
        Err(VmError::HostDenied { detail: format!("host_access_denied:write:{GAME_HEALTH_COMPONENT}") })
    }
}

fn gate_health_read(shared: &EngineShared) -> Result<(), VmError> {
    if !shared.execution_profile.enforces_runtime_gate() {
        return Ok(());
    }
    if !shared.access.allows_read_world() {
        return Err(VmError::HostDenied { detail: "host_access_denied:read_world".into() });
    }
    if !shared.access.allows_read_component(GAME_HEALTH_COMPONENT) {
        return Err(VmError::HostDenied { detail: format!("host_access_denied:read:{GAME_HEALTH_COMPONENT}") });
    }
    Ok(())
}
