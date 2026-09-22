//! 编译期与运行期共用的结构化宿主 ABI。
//!
//! 正式路径一律使用 [`HostSchema`] 与 [`HostBindTable`]：效果、能力、确定性与槽位
//! 在编译期绑定；链接与制品按 [`HostFunctionId`] 限定名校验，字节码按槽位发射 [`spark_vm::Op::CallHost`]。

use std::sync::Arc;

use spark_ir::{DeterminismKind, HostBindEntry, HostBindTable, HostCompilePolicy, HostEffectKind, HostId, HostPhaseKind};
use spark_script_valkyrie::{NativeParam, TypeRef};

/// 宿主调用的效果分类（编译器内部约束，不必全部暴露为用户语法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostEffect {
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

/// 确定性分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DeterminismClass {
    Deterministic,
    FrameLocal,
    #[default]
    Nondeterministic,
}

/// 允许执行的生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HostPhase {
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

/// 挂起行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuspensionBehavior {
    Forbidden,
    Allowed,
}

/// 线程亲和。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThreadAffinity {
    MainOnly,
    AnyWorker,
}

/// 错误模型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostErrorModel {
    /// 失败以 trap / 异常边表达。
    Trap,
    /// 返回可区分的错误值。
    ResultValue,
    /// 永不失败（仍须在文档中诚实）。
    Infallible,
}

/// 能力标识（稳定字符串；后续可 intern）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilityId {
    pub path: Arc<str>,
}

impl CapabilityId {
    pub fn new(path: impl Into<Arc<str>>) -> Self {
        Self { path: path.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }
}

impl From<&str> for CapabilityId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// 宿主函数稳定身份（链接与装载用，不是调试显示名）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostFunctionId {
    pub namespace: Arc<str>,
    pub name: Arc<str>,
    pub abi_version: u32,
}

impl HostFunctionId {
    pub fn new(namespace: impl Into<Arc<str>>, name: impl Into<Arc<str>>, abi_version: u32) -> Self {
        Self { namespace: namespace.into(), name: name.into(), abi_version }
    }

    /// 调试 / 诊断用限定名：`namespace.name`。
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }
}

/// 单个宿主函数的完整 schema 条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFunction {
    pub id: HostFunctionId,
    pub params: Vec<NativeParam>,
    pub return_ty: Option<TypeRef>,
    pub error_model: HostErrorModel,
    pub effects: Vec<HostEffect>,
    pub allowed_phases: Vec<HostPhase>,
    pub required_capabilities: Vec<CapabilityId>,
    pub determinism: DeterminismClass,
    pub thread_affinity: ThreadAffinity,
    pub suspension: SuspensionBehavior,
    pub docs: Option<Arc<str>>,
}

impl HostFunction {
    /// 以默认保守约束新建条目（纯、确定性、禁止挂起、任意阶段）。
    pub fn new(id: HostFunctionId) -> Self {
        Self {
            id,
            params: Vec::new(),
            return_ty: None,
            error_model: HostErrorModel::Trap,
            effects: vec![HostEffect::Pure],
            allowed_phases: vec![HostPhase::Any],
            required_capabilities: Vec::new(),
            determinism: DeterminismClass::Deterministic,
            thread_affinity: ThreadAffinity::MainOnly,
            suspension: SuspensionBehavior::Forbidden,
            docs: None,
        }
    }

    pub fn param(mut self, param: NativeParam) -> Self {
        self.params.push(param);
        self
    }

    pub fn returns(mut self, ty: impl Into<TypeRef>) -> Self {
        self.return_ty = Some(ty.into());
        self
    }

    pub fn with_docs(mut self, docs: impl Into<Arc<str>>) -> Self {
        self.docs = Some(docs.into());
        self
    }

    pub fn effect(mut self, effect: HostEffect) -> Self {
        if !self.effects.contains(&effect) {
            self.effects.push(effect);
        }
        // 含 Pure 以外效果时去掉 Pure，避免误导。
        if effect != HostEffect::Pure {
            self.effects.retain(|e| *e != HostEffect::Pure);
        }
        self
    }

    pub fn capability(mut self, cap: impl Into<CapabilityId>) -> Self {
        let cap = cap.into();
        if !self.required_capabilities.iter().any(|c| c == &cap) {
            self.required_capabilities.push(cap);
        }
        self
    }

    pub fn phases(mut self, phases: impl IntoIterator<Item = HostPhase>) -> Self {
        self.allowed_phases = phases.into_iter().collect();
        self
    }

    pub fn determinism(mut self, class: DeterminismClass) -> Self {
        self.determinism = class;
        self
    }

    pub fn suspension(mut self, behavior: SuspensionBehavior) -> Self {
        self.suspension = behavior;
        self
    }

    /// 脚本侧未限定命名空间时的名字（`id.name`）。
    pub fn short_name(&self) -> &str {
        self.id.name.as_ref()
    }

    /// 当前调度阶段是否允许调用本函数。
    ///
    /// `HostPhase::Any` 作为调用方阶段表示“未声明阶段”（如钩子 `call`），一律放行。
    /// 函数侧含 `Any` 或空列表也表示不限制。
    pub fn allows_phase(&self, phase: HostPhase) -> bool {
        if phase == HostPhase::Any {
            return true;
        }
        if self.allowed_phases.is_empty() {
            return true;
        }
        self.allowed_phases.iter().any(|p| *p == HostPhase::Any || *p == phase)
    }
}

/// 一整份宿主 ABI schema（编译与运行必须同一份）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostSchema {
    pub abi_version: u32,
    pub functions: Vec<HostFunction>,
}

impl HostSchema {
    pub fn new(abi_version: u32) -> Self {
        Self { abi_version, functions: Vec::new() }
    }

    pub fn insert(&mut self, func: HostFunction) {
        if let Some(existing) = self.functions.iter_mut().find(|f| f.id == func.id) {
            *existing = func;
        }
        else {
            self.functions.push(func);
        }
    }

    pub fn get(&self, id: &HostFunctionId) -> Option<&HostFunction> {
        self.functions.iter().find(|f| f.id == *id)
    }

    /// 解析宿主导入名：优先完整 [`HostFunctionId::qualified_name`]，否则要求短名全局唯一。
    pub fn resolve_import(&self, import: &str) -> Result<&HostFunction, String> {
        if let Some(f) = self.functions.iter().find(|f| f.id.qualified_name() == import) {
            return Ok(f);
        }
        self.resolve_short_name(import)
    }

    pub fn get_by_short_name(&self, name: &str) -> Option<&HostFunction> {
        self.resolve_short_name(name).ok()
    }

    /// 按短名解析；冲突或缺失时返回错误令牌。
    pub fn resolve_short_name(&self, name: &str) -> Result<&HostFunction, String> {
        let matches: Vec<_> = self.functions.iter().filter(|f| f.id.name.as_ref() == name).collect();
        match matches.as_slice() {
            [f] => Ok(f),
            [] => Err(format!("host_unknown:{name}")),
            _ => Err(format!("host_short_name_conflict:{name}")),
        }
    }

    /// 导出编译期 [`HostBindTable`]（短名冲突即失败）。
    pub fn to_bind_table(&self) -> Result<HostBindTable, String> {
        self.to_bind_table_with_policy(HostCompilePolicy::open())
    }

    /// 带编译策略导出绑定表（能力 / 确定性来自 [`CompilationRequest`]）。
    pub fn to_bind_table_with_policy(&self, policy: HostCompilePolicy) -> Result<HostBindTable, String> {
        let mut table = HostBindTable::new().with_policy(policy);
        for (i, func) in self.functions.iter().enumerate() {
            let param_count = if func.params.is_empty() {
                // 空参数表 = 未声明 arity（桩 / 动态脚本）；非空才做个数检查。
                u16::MAX
            }
            else {
                func.params.len() as u16
            };
            let param_tys = func.params.iter().map(|p| Arc::clone(&p.ty.path)).collect();
            let return_ty = func.return_ty.as_ref().map(|t| Arc::clone(&t.path));
            table.push(HostBindEntry {
                id: HostId::new(Arc::clone(&func.id.namespace), Arc::clone(&func.id.name), func.id.abi_version),
                slot: i as u32,
                param_count,
                param_tys,
                return_ty,
                effects: func.effects.iter().map(|e| map_effect(*e)).collect(),
                allowed_phases: func.allowed_phases.iter().map(|p| map_phase(*p)).collect(),
                required_capabilities: func.required_capabilities.iter().map(|c| Arc::clone(&c.path)).collect(),
                determinism: map_determinism(func.determinism),
            })?;
        }
        Ok(table)
    }

    /// 链接后的稳定槽位下标（按插入顺序）。
    pub fn slot_of(&self, id: &HostFunctionId) -> Option<u32> {
        self.functions.iter().position(|f| f.id == *id).map(|i| i as u32)
    }

    /// 按短名查找槽位（短名须唯一）。
    pub fn slot_of_short_name(&self, name: &str) -> Option<u32> {
        let f = self.resolve_short_name(name).ok()?;
        self.slot_of(&f.id)
    }

    /// 制品 / 链接用的稳定键：[`HostFunctionId::qualified_name`] 顺序 = 槽位。
    pub fn qualified_names(&self) -> Vec<String> {
        self.functions.iter().map(|f| f.id.qualified_name()).collect()
    }

    /// VM `register_native` / `prepare_host_slots` 调度名（限定名，与槽位同序）。
    pub fn dispatch_names(&self) -> Vec<String> {
        self.qualified_names()
    }

    /// schema 内容指纹（缓存键 / 装载校验用）。
    pub fn content_hash(&self) -> u64 {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };
        let mut h = DefaultHasher::new();
        self.abi_version.hash(&mut h);
        for f in &self.functions {
            f.id.namespace.hash(&mut h);
            f.id.name.hash(&mut h);
            f.id.abi_version.hash(&mut h);
            for p in &f.params {
                p.name.hash(&mut h);
                p.ty.path.hash(&mut h);
            }
            if let Some(ty) = &f.return_ty {
                ty.path.hash(&mut h);
            }
            for e in &f.effects {
                (*e as u8).hash(&mut h);
            }
            for c in &f.required_capabilities {
                c.path.hash(&mut h);
            }
        }
        h.finish()
    }
}

fn map_effect(e: HostEffect) -> HostEffectKind {
    match e {
        HostEffect::Pure => HostEffectKind::Pure,
        HostEffect::ReadWorld => HostEffectKind::ReadWorld,
        HostEffect::WriteComponent => HostEffectKind::WriteComponent,
        HostEffect::SpawnEntity => HostEffectKind::SpawnEntity,
        HostEffect::DespawnEntity => HostEffectKind::DespawnEntity,
        HostEffect::AssetRead => HostEffectKind::AssetRead,
        HostEffect::AudioEmit => HostEffectKind::AudioEmit,
        HostEffect::NetworkSend => HostEffectKind::NetworkSend,
        HostEffect::Nondeterministic => HostEffectKind::Nondeterministic,
        HostEffect::Suspend => HostEffectKind::Suspend,
        HostEffect::EditorOnly => HostEffectKind::EditorOnly,
    }
}

fn map_phase(p: HostPhase) -> HostPhaseKind {
    match p {
        HostPhase::OnLoad => HostPhaseKind::OnLoad,
        HostPhase::OnStart => HostPhaseKind::OnStart,
        HostPhase::FixedUpdate => HostPhaseKind::FixedUpdate,
        HostPhase::Update => HostPhaseKind::Update,
        HostPhase::LateUpdate => HostPhaseKind::LateUpdate,
        HostPhase::RenderPrepare => HostPhaseKind::RenderPrepare,
        HostPhase::OnEvent => HostPhaseKind::OnEvent,
        HostPhase::OnUnload => HostPhaseKind::OnUnload,
        HostPhase::Any => HostPhaseKind::Any,
    }
}

fn map_determinism(d: DeterminismClass) -> DeterminismKind {
    match d {
        DeterminismClass::Deterministic => DeterminismKind::Deterministic,
        DeterminismClass::FrameLocal => DeterminismKind::FrameLocal,
        DeterminismClass::Nondeterministic => DeterminismKind::Nondeterministic,
    }
}

/// 由编译请求构造绑定策略。
pub fn compile_policy_from_request(request: &crate::request::CompilationRequest) -> HostCompilePolicy {
    HostCompilePolicy {
        granted_capabilities: request.required_capabilities.iter().map(|c| Arc::clone(&c.path)).collect(),
        determinism: map_determinism(request.determinism),
    }
}
