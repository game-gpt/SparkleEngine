//! 宿主绑定表：HIR / MIR / codegen 共用的稳定身份、槽位与 ABI 约束。
//!
//! 短名仅用于 REPL 与诊断解析；链接与字节码一律按 [`HostId`] 对应槽位。

use std::collections::HashMap;
use std::sync::Arc;

/// 宿主函数稳定身份（与 `spark-script::HostFunctionId` 字段对齐）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostId {
    pub namespace: Arc<str>,
    pub name: Arc<str>,
    pub abi_version: u32,
}

impl HostId {
    pub fn new(
        namespace: impl Into<Arc<str>>,
        name: impl Into<Arc<str>>,
        abi_version: u32,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            abi_version,
        }
    }

    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }

    pub fn short_name(&self) -> &str {
        self.name.as_ref()
    }
}

/// 宿主效果（IR 侧，与 `spark-script::HostEffect` 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostEffectKind {
    Pure,
    ReadWorld,
    WriteComponent,
    SpawnEntity,
    DespawnEntity,
    AssetRead,
    AudioEmit,
    NetworkSend,
    Nondeterministic,
    Suspend,
    EditorOnly,
}

/// 允许阶段（IR 侧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HostPhaseKind {
    OnLoad,
    OnStart,
    FixedUpdate,
    Update,
    LateUpdate,
    RenderPrepare,
    OnEvent,
    OnUnload,
    #[default]
    Any,
}

/// 确定性等级（IR 侧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DeterminismKind {
    Deterministic,
    FrameLocal,
    #[default]
    Nondeterministic,
}

impl DeterminismKind {
    /// `self` 作为编译/System 要求时，是否允许调用 `host` 级函数。
    pub fn allows_host(self, host: DeterminismKind) -> bool {
        match self {
            Self::Nondeterministic => true,
            Self::FrameLocal => !matches!(host, Self::Nondeterministic),
            Self::Deterministic => matches!(host, Self::Deterministic),
        }
    }
}

/// 编译期策略：能力授予与确定性上限（来自 `CompilationRequest`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostCompilePolicy {
    pub granted_capabilities: Vec<Arc<str>>,
    pub determinism: DeterminismKind,
}

impl HostCompilePolicy {
    pub fn open() -> Self {
        Self::default()
    }

    pub fn grants(&self, cap: &str) -> bool {
        self.granted_capabilities.is_empty()
            || self.granted_capabilities.iter().any(|c| c.as_ref() == cap)
    }
}

/// 绑定表中的一条宿主导入（含完整 ABI 快照）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBindEntry {
    pub id: HostId,
    /// 与 schema 插入顺序一致的稳定槽位。
    pub slot: u32,
    /// 形参个数；`u16::MAX` 表示未知（跳过 arity 检查）。
    pub param_count: u16,
    /// 形参类型路径（与 `NativeParam.ty` 对齐；空 = 未声明）。
    pub param_tys: Vec<Arc<str>>,
    pub return_ty: Option<Arc<str>>,
    pub effects: Vec<HostEffectKind>,
    pub allowed_phases: Vec<HostPhaseKind>,
    pub required_capabilities: Vec<Arc<str>>,
    pub determinism: DeterminismKind,
}

impl HostBindEntry {
    /// 最小桩（短名列表 / 测试）。
    pub fn stub(id: HostId, slot: u32) -> Self {
        Self {
            id,
            slot,
            param_count: u16::MAX,
            param_tys: Vec::new(),
            return_ty: None,
            effects: vec![HostEffectKind::Pure],
            allowed_phases: vec![HostPhaseKind::Any],
            required_capabilities: Vec::new(),
            determinism: DeterminismKind::Nondeterministic,
        }
    }

    /// 校验一次宿主调用是否满足绑定表策略与 arity。
    pub fn check_call(&self, argc: usize, policy: &HostCompilePolicy) -> Result<(), String> {
        if self.param_count != u16::MAX && argc as u16 != self.param_count {
            return Err(format!(
                "host_arity:{}:expected_{}_got_{}",
                self.id.qualified_name(),
                self.param_count,
                argc
            ));
        }
        for cap in &self.required_capabilities {
            if !policy.grants(cap.as_ref()) {
                return Err(format!(
                    "host_capability_denied:{}:{}",
                    self.id.qualified_name(),
                    cap
                ));
            }
        }
        if !policy.determinism.allows_host(self.determinism) {
            return Err(format!(
                "host_determinism_denied:{}:policy={:?}:host={:?}",
                self.id.qualified_name(),
                policy.determinism,
                self.determinism
            ));
        }
        Ok(())
    }
}

/// 编译期宿主绑定表（拒绝短名冲突）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostBindTable {
    entries: Vec<HostBindEntry>,
    by_qualified: HashMap<String, u32>,
    /// 短名 → 槽位；仅当该短名全局唯一时存在。
    by_short: HashMap<Arc<str>, u32>,
    pub policy: HostCompilePolicy,
}

impl HostBindTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: HostCompilePolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn set_policy(&mut self, policy: HostCompilePolicy) {
        self.policy = policy;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[HostBindEntry] {
        &self.entries
    }

    /// 按 schema 插入顺序追加；同短名第二次出现则报错。
    pub fn push(&mut self, entry: HostBindEntry) -> Result<(), String> {
        let short = Arc::clone(&entry.id.name);
        if self.by_short.contains_key(&short) {
            return Err(format!("host_short_name_conflict:{}", short));
        }
        let q = entry.id.qualified_name();
        if self.by_qualified.contains_key(&q) {
            return Err(format!("host_id_conflict:{q}"));
        }
        let slot = entry.slot;
        if slot as usize != self.entries.len() {
            return Err(format!(
                "host_slot_gap:expected_{}_got_{}",
                self.entries.len(),
                slot
            ));
        }
        self.by_qualified.insert(q, slot);
        self.by_short.insert(short, slot);
        self.entries.push(entry);
        Ok(())
    }

    /// 由短名列表构建（默认 namespace=`host`，abi=`1`）。重复短名失败。
    pub fn from_short_names(names: &[&str]) -> Result<Self, String> {
        let mut table = Self::new();
        for (i, name) in names.iter().enumerate() {
            table.push(HostBindEntry::stub(HostId::new("host", *name, 1), i as u32))?;
        }
        Ok(table)
    }

    /// 解析脚本中的宿主调用名：优先 `namespace.name`，否则唯一短名。
    pub fn resolve(&self, name: &str) -> Result<&HostBindEntry, String> {
        if let Some(slot) = self.by_qualified.get(name) {
            return Ok(&self.entries[*slot as usize]);
        }
        if let Some(slot) = self.by_short.get(name) {
            return Ok(&self.entries[*slot as usize]);
        }
        Err(format!("host_unknown:{name}"))
    }

    /// 解析并按策略校验调用。
    pub fn resolve_call(&self, name: &str, argc: usize) -> Result<&HostBindEntry, String> {
        let entry = self.resolve(name)?;
        entry.check_call(argc, &self.policy)?;
        Ok(entry)
    }

    pub fn get(&self, id: &HostId) -> Option<&HostBindEntry> {
        let q = id.qualified_name();
        let slot = *self.by_qualified.get(&q)?;
        self.entries.get(slot as usize)
    }

    /// 槽位诊断名（顺序 = 槽位）；VM `prepare_host_slots` 仍按此顺序注册。
    pub fn slot_names(&self) -> Vec<&str> {
        self.entries
            .iter()
            .map(|e| e.id.short_name())
            .collect()
    }

    pub fn contains_short(&self, name: &str) -> bool {
        self.by_short.contains_key(name) || self.by_qualified.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_call_checks_arity_and_capability() {
        let mut table = HostBindTable::new().with_policy(HostCompilePolicy {
            granted_capabilities: vec![Arc::from("ecs.command")],
            determinism: DeterminismKind::Deterministic,
        });
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
        assert!(table
            .resolve_call("spawn", 1)
            .unwrap_err()
            .contains("host_capability_denied"));
    }

    #[test]
    fn determinism_policy_rejects_nondeterministic_host() {
        let mut table = HostBindTable::new().with_policy(HostCompilePolicy {
            granted_capabilities: Vec::new(),
            determinism: DeterminismKind::Deterministic,
        });
        let mut entry = HostBindEntry::stub(HostId::new("log", "print", 1), 0);
        entry.determinism = DeterminismKind::Nondeterministic;
        table.push(entry).unwrap();
        assert!(table
            .resolve_call("print", 0)
            .unwrap_err()
            .contains("host_determinism_denied"));
    }
}
