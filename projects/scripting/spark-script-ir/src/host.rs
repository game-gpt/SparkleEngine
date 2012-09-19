//! 宿主绑定表：HIR / MIR / codegen 共用的稳定身份与槽位。
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

/// 绑定表中的一条宿主导入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBindEntry {
    pub id: HostId,
    /// 与 schema 插入顺序一致的稳定槽位。
    pub slot: u32,
    /// 形参个数；`u16::MAX` 表示未知（跳过 arity 检查）。
    pub param_count: u16,
}

/// 编译期宿主绑定表（拒绝短名冲突）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostBindTable {
    entries: Vec<HostBindEntry>,
    by_qualified: HashMap<String, u32>,
    /// 短名 → 槽位；仅当该短名全局唯一时存在。
    by_short: HashMap<Arc<str>, u32>,
}

impl HostBindTable {
    pub fn new() -> Self {
        Self::default()
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
            table.push(HostBindEntry {
                id: HostId::new("host", *name, 1),
                slot: i as u32,
                param_count: u16::MAX,
            })?;
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
