//! 自 `src/host.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_ir::*;
use std::sync::Arc;

#[test]
fn resolve_call_checks_arity_and_capability() {
    let mut table = HostBindTable::new()
        .with_policy(HostCompilePolicy { granted_capabilities: vec![Arc::from("ecs.command")], determinism: DeterminismKind::Deterministic });
    let mut entry = HostBindEntry::stub(HostId::new("ecs", "spawn", 1), 0);
    entry.param_count = 1;
    entry.required_capabilities = vec![Arc::from("ecs.command")];
    entry.determinism = DeterminismKind::Deterministic;
    entry.effects = vec![HostEffectKind::SpawnEntity];
    table.push(entry).unwrap();

    assert!(table.resolve_call("spawn", 1).is_ok());
    assert!(table.resolve_call("spawn", 2).unwrap_err().contains("host_arity"));

    table.policy.granted_capabilities.clear();
    // 空授予列表 = 开放（REPL）；显式清空后再设非匹配
    table.policy.granted_capabilities = vec![Arc::from("other")];
    assert!(table.resolve_call("spawn", 1).unwrap_err().contains("host_capability_denied"));
}

#[test]
fn determinism_policy_rejects_nondeterministic_host() {
    let mut table =
        HostBindTable::new().with_policy(HostCompilePolicy { granted_capabilities: Vec::new(), determinism: DeterminismKind::Deterministic });
    let mut entry = HostBindEntry::stub(HostId::new("log", "print", 1), 0);
    entry.determinism = DeterminismKind::Nondeterministic;
    table.push(entry).unwrap();
    assert!(table.resolve_call("print", 0).unwrap_err().contains("host_determinism_denied"));
}
