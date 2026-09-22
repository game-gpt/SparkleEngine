//! 宿主绑定表：HIR / MIR / codegen 共用的稳定身份、槽位与 ABI 约束。
//!
//! 脚本源码可写未限定短名；解析时映射到唯一 [`HostId`] 槽位。制品与链接按限定名，字节码按槽位。

use std::{collections::HashMap, sync::Arc};

/// 宿主函数稳定身份（与 `spark-script::HostFunctionId` 字段对齐）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostId {
    /// 命名空间（如 `spark.world`）。
    pub namespace: Arc<str>,
    /// 短名（命名空间内唯一）。
    pub name: Arc<str>,
    /// ABI 版本；不匹配则不得链接。
    pub abi_version: u32,
}

impl HostId {
    /// 构造稳定身份。
    pub fn new(namespace: impl Into<Arc<str>>, name: impl Into<Arc<str>>, abi_version: u32) -> Self {
        Self { namespace: namespace.into(), name: name.into(), abi_version }
    }

    /// `namespace.name` 限定名（制品 / 链接键）。
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }

    /// 短名视图。
    pub fn short_name(&self) -> &str {
        self.name.as_ref()
    }
}

/// 宿主效果（IR 侧，与 `spark-script::HostEffect` 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostEffectKind {
    /// 无世界副作用（可重排 / 缓存）。
    Pure,
    /// 只读查询世界状态。
    ReadWorld,
    /// 写入 ECS 组件。
    WriteComponent,
    /// 生成实体。
    SpawnEntity,
    /// 销毁实体。
    DespawnEntity,
    /// 读取资产字节 / 句柄。
    AssetRead,
    /// 发出音频请求。
    AudioEmit,
    /// 发送网络消息。
    NetworkSend,
    /// 非确定性（随机、墙钟、外部 IO 等）。
    Nondeterministic,
    /// 挂起脚本协程 / await 点。
    Suspend,
    /// 仅编辑器可用；运行时链接应拒绝。
    EditorOnly,
}

/// 允许阶段（IR 侧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HostPhaseKind {
    /// 模块装载。
    OnLoad,
    /// 首次进入运行。
    OnStart,
    /// 固定步仿真。
    FixedUpdate,
    /// 可变帧更新。
    Update,
    /// 帧末更新。
    LateUpdate,
    /// 渲染提交前。
    RenderPrepare,
    /// 事件回调。
    OnEvent,
    /// 卸载。
    OnUnload,
    /// 任意阶段均可。
    #[default]
    Any,
}

/// 确定性等级（IR 侧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DeterminismKind {
    /// 跨端可复现（禁止墙钟 / 真随机等）。
    Deterministic,
    /// 单帧内可复现即可。
    FrameLocal,
    /// 允许非确定性宿主。
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
    /// 已授予能力 token；空表表示开放（测试便利）。
    pub granted_capabilities: Vec<Arc<str>>,
    /// 本编译单元允许的最高确定性档。
    pub determinism: DeterminismKind,
}

impl HostCompilePolicy {
    /// 开放策略：任意能力、默认确定性档。
    pub fn open() -> Self {
        Self::default()
    }

    /// 是否授予 `cap`；`granted_capabilities` 为空时一律放行。
    pub fn grants(&self, cap: &str) -> bool {
        self.granted_capabilities.is_empty() || self.granted_capabilities.iter().any(|c| c.as_ref() == cap)
    }
}

/// 绑定表中的一条宿主导入（含完整 ABI 快照）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBindEntry {
    /// 稳定身份。
    pub id: HostId,
    /// 与 schema 插入顺序一致的稳定槽位。
    pub slot: u32,
    /// 形参个数；`u16::MAX` 表示未知（跳过 arity 检查）。
    pub param_count: u16,
    /// 形参类型路径（与 `NativeParam.ty` 对齐；空 = 未声明）。
    pub param_tys: Vec<Arc<str>>,
    /// 返回类型路径；`None` 表示 void / 未声明。
    pub return_ty: Option<Arc<str>>,
    /// 声明的效果集合。
    pub effects: Vec<HostEffectKind>,
    /// 允许调用的帧阶段。
    pub allowed_phases: Vec<HostPhaseKind>,
    /// 调用所需能力 token。
    pub required_capabilities: Vec<Arc<str>>,
    /// 该宿主自身的确定性档。
    pub determinism: DeterminismKind,
}

impl HostBindEntry {
    /// 最小测试桩（完整 [`HostId`]，未知 arity）。
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
            return Err(format!("host_arity:{}:expected_{}_got_{}", self.id.qualified_name(), self.param_count, argc));
        }
        for cap in &self.required_capabilities {
            if !policy.grants(cap.as_ref()) {
                return Err(format!("host_capability_denied:{}:{}", self.id.qualified_name(), cap));
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
    /// 当前编译策略（能力 / 确定性）。
    pub policy: HostCompilePolicy,
}

impl HostBindTable {
    /// 空表 + 默认开放策略。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置策略并返回 `self`（构建期链式写法）。
    pub fn with_policy(mut self, policy: HostCompilePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// 替换编译策略。
    pub fn set_policy(&mut self, policy: HostCompilePolicy) {
        self.policy = policy;
    }

    /// 已登记宿主条数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否无任何宿主。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按槽位顺序的条目切片。
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
            return Err(format!("host_slot_gap:expected_{}_got_{}", self.entries.len(), slot));
        }
        self.by_qualified.insert(q, slot);
        self.by_short.insert(short, slot);
        self.entries.push(entry);
        Ok(())
    }

    /// 由 [`HostId`] 列表构建测试桩（插入顺序 = 槽位）。重复短名或限定名失败。
    pub fn from_ids(ids: impl IntoIterator<Item = HostId>) -> Result<Self, String> {
        let mut table = Self::new();
        for (i, id) in ids.into_iter().enumerate() {
            table.push(HostBindEntry::stub(id, i as u32))?;
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

    /// 按完整 [`HostId`] 查找条目。
    pub fn get(&self, id: &HostId) -> Option<&HostBindEntry> {
        let q = id.qualified_name();
        let slot = *self.by_qualified.get(&q)?;
        self.entries.get(slot as usize)
    }

    /// 槽位诊断名（限定名，顺序 = 槽位）。
    pub fn slot_names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.id.qualified_name()).collect()
    }

    /// VM `prepare_host_slots` 调度名（限定名，与 `register_native` 键一致）。
    pub fn dispatch_names(&self) -> Vec<String> {
        self.slot_names()
    }

    /// 短名或限定名是否已登记。
    pub fn contains_short(&self, name: &str) -> bool {
        self.by_short.contains_key(name) || self.by_qualified.contains_key(name)
    }
}
