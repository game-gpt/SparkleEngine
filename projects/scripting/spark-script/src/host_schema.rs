//! 编译期与运行期共用的结构化宿主 ABI。
//!
//! 正式路径一律使用 [`HostSchema`] 与 [`HostBindTable`]：效果、能力、确定性与槽位
//! 在编译期绑定；链接与制品按 [`HostFunctionId`] 限定名校验，字节码按槽位发射 [`spark_vm::Op::CallHost`]。

use std::sync::Arc;

use spark_ir::{DeterminismKind, HostBindEntry, HostBindTable, HostCompilePolicy, HostEffectKind, HostId, HostPhaseKind};
use spark_script_valkyrie::{NativeParam, TypeRef};

/// 宿主调用的效果分类（编译器内部约束，不必全部暴露为用户语法）。
///
/// 写入 [`HostBindEntry::effects`] 后，编译策略与字节码验证会拒绝越权调用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostEffect {
    /// 无世界副作用；可与确定性路径组合。
    Pure,
    /// 只读查询世界 / 组件状态。
    ReadWorld,
    /// 写入组件字段（可变世界）。
    WriteComponent,
    /// 生成实体。
    SpawnEntity,
    /// 销毁实体。
    DespawnEntity,
    /// 读取资产管线（可能触发 I/O 或缓存填充）。
    AssetRead,
    /// 发出音频事件。
    AudioEmit,
    /// 发送网络报文。
    NetworkSend,
    /// 显式非确定性（随机、墙钟、外部输入等）。
    Nondeterministic,
    /// 允许协作式挂起（协程 / await 语义）。
    Suspend,
    /// 仅编辑器宿主可用；运行时装载应拒绝。
    EditorOnly,
}

/// 确定性分类。
///
/// 与 [`CompilationRequest::determinism`] 策略比对：策略要求确定性时，绑定表不得含更弱类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DeterminismClass {
    /// 相同输入与种子下结果可复现。
    Deterministic,
    /// 仅在同一帧内可复现（跨帧可漂移）。
    FrameLocal,
    /// 不保证复现（默认）。
    #[default]
    Nondeterministic,
}

/// 允许执行的生命周期阶段。
///
/// [`HostFunction::allows_phase`]：调用方为 [`HostPhase::Any`]（未声明阶段）一律放行；
/// 函数侧含 `Any` 或空列表表示不限制。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HostPhase {
    /// 模组 / 脚本装载钩子。
    OnLoad,
    /// 首次进入可玩状态。
    OnStart,
    /// 固定步长模拟。
    FixedUpdate,
    /// 可变帧更新。
    Update,
    /// 更新后的收尾阶段。
    LateUpdate,
    /// 渲染提交前准备。
    RenderPrepare,
    /// 事件驱动回调。
    OnEvent,
    /// 卸载 / 析构钩子。
    OnUnload,
    /// 不限制阶段（默认）。
    #[default]
    Any,
}

/// 挂起行为。
///
/// 含 [`HostEffect::Suspend`] 的函数通常应设为 [`SuspensionBehavior::Allowed`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuspensionBehavior {
    /// 禁止挂起；调用点不得生成挂起点。
    Forbidden,
    /// 允许协作式挂起。
    Allowed,
}

/// 线程亲和。
///
/// 运行时调度应尊重此约束；编译期目前主要作文档与策略元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThreadAffinity {
    /// 仅主线程可调用。
    MainOnly,
    /// 任意工作线程可调用。
    AnyWorker,
}

/// 错误模型。
///
/// 决定失败如何从宿主传回脚本：trap、结果值或宣称永不失败。
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
///
/// 路径形如 `world.write`；编译策略的 `granted_capabilities` 必须覆盖函数所需能力。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilityId {
    /// 稳定能力路径（比较与哈希均按此字符串）。
    pub path: Arc<str>,
}

impl CapabilityId {
    /// 由路径字符串构造能力标识。
    pub fn new(path: impl Into<Arc<str>>) -> Self {
        Self { path: path.into() }
    }

    /// 返回能力路径的字符串切片。
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
///
/// 限定名 `namespace.name` 与 `abi_version` 共同决定槽位身份；ABI 升级须改版本号以免静默错绑。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostFunctionId {
    /// 命名空间（如 `spark.world`）。
    pub namespace: Arc<str>,
    /// 函数短名（如 `spawn`）；短名解析要求全局唯一。
    pub name: Arc<str>,
    /// 该函数自身的 ABI 版本（可与 schema 级 `abi_version` 独立演进）。
    pub abi_version: u32,
}

impl HostFunctionId {
    /// 构造稳定身份三元组。
    pub fn new(namespace: impl Into<Arc<str>>, name: impl Into<Arc<str>>, abi_version: u32) -> Self {
        Self { namespace: namespace.into(), name: name.into(), abi_version }
    }

    /// 调试 / 诊断用限定名：`namespace.name`（不含 ABI 版本）。
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }
}

/// 单个宿主函数的完整 schema 条目。
///
/// 编译期经 [`HostSchema::to_bind_table_with_policy`] 降为 [`HostBindEntry`]；
/// 运行期按插入顺序得到 `CallHost` 槽位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFunction {
    /// 稳定身份（链接键）。
    pub id: HostFunctionId,
    /// 形参表；空表表示未声明 arity（桩 / 动态脚本），绑定时 `param_count = u16::MAX`。
    pub params: Vec<NativeParam>,
    /// 返回类型；`None` 表示无返回值或不声明。
    pub return_ty: Option<TypeRef>,
    /// 失败如何传回脚本。
    pub error_model: HostErrorModel,
    /// 效果集合；含非 `Pure` 时构建器会去掉误导性的 `Pure`。
    pub effects: Vec<HostEffect>,
    /// 允许调用的生命周期阶段；空或含 [`HostPhase::Any`] 表示不限制。
    pub allowed_phases: Vec<HostPhase>,
    /// 调用前宿主必须授予的能力。
    pub required_capabilities: Vec<CapabilityId>,
    /// 该函数结果的确定性类。
    pub determinism: DeterminismClass,
    /// 允许执行的线程亲和。
    pub thread_affinity: ThreadAffinity,
    /// 是否允许协作式挂起。
    pub suspension: SuspensionBehavior,
    /// 面向工具 / 诊断的可选说明文本（不进字节码）。
    pub docs: Option<Arc<str>>,
}

impl HostFunction {
    /// 以默认保守约束新建条目（纯、确定性、禁止挂起、任意阶段、主线程、Trap）。
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

    /// 追加一个形参（构建器链式 API）。
    pub fn param(mut self, param: NativeParam) -> Self {
        self.params.push(param);
        self
    }

    /// 声明返回类型。
    pub fn returns(mut self, ty: impl Into<TypeRef>) -> Self {
        self.return_ty = Some(ty.into());
        self
    }

    /// 附加工具可见文档字符串。
    pub fn with_docs(mut self, docs: impl Into<Arc<str>>) -> Self {
        self.docs = Some(docs.into());
        self
    }

    /// 追加效果；若加入非 [`HostEffect::Pure`]，会移除已有的 `Pure`，避免「纯 + 副作用」矛盾。
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

    /// 追加所需能力（去重）。
    pub fn capability(mut self, cap: impl Into<CapabilityId>) -> Self {
        let cap = cap.into();
        if !self.required_capabilities.iter().any(|c| c == &cap) {
            self.required_capabilities.push(cap);
        }
        self
    }

    /// 覆盖允许的生命周期阶段列表。
    pub fn phases(mut self, phases: impl IntoIterator<Item = HostPhase>) -> Self {
        self.allowed_phases = phases.into_iter().collect();
        self
    }

    /// 设置确定性类。
    pub fn determinism(mut self, class: DeterminismClass) -> Self {
        self.determinism = class;
        self
    }

    /// 设置挂起行为。
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
///
/// 不变式：`functions` 插入顺序即槽位下标；`content_hash` / `abi_version` 写入制品，
/// 装载时与运行时 schema 比对（见 [`crate::artifact::ExecutableImage::check_host_schema`]）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostSchema {
    /// Schema 级 ABI 版本（制品头校验；与单函数 `id.abi_version` 独立）。
    pub abi_version: u32,
    /// 宿主函数表；下标 = `CallHost` 槽位。
    pub functions: Vec<HostFunction>,
}

impl HostSchema {
    /// 创建空 schema，并固定 schema 级 ABI 版本。
    pub fn new(abi_version: u32) -> Self {
        Self { abi_version, functions: Vec::new() }
    }

    /// 按 [`HostFunctionId`] 插入或替换条目（同 id 覆盖，保持其余槽位相对顺序）。
    pub fn insert(&mut self, func: HostFunction) {
        if let Some(existing) = self.functions.iter_mut().find(|f| f.id == func.id) {
            *existing = func;
        }
        else {
            self.functions.push(func);
        }
    }

    /// 按稳定身份查找条目。
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

    /// 按短名查找；冲突或缺失时返回 `None`（不暴露错误令牌）。
    pub fn get_by_short_name(&self, name: &str) -> Option<&HostFunction> {
        self.resolve_short_name(name).ok()
    }

    /// 按短名解析；冲突或缺失时返回错误令牌。
    ///
    /// 错误串：`host_unknown:{name}` / `host_short_name_conflict:{name}`。
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
///
/// 将 [`CompilationRequest::required_capabilities`] 与 [`CompilationRequest::determinism`]
/// 映射为 [`HostCompilePolicy`]，供 [`HostSchema::to_bind_table_with_policy`] 使用。
pub fn compile_policy_from_request(request: &crate::request::CompilationRequest) -> HostCompilePolicy {
    HostCompilePolicy {
        granted_capabilities: request.required_capabilities.iter().map(|c| Arc::clone(&c.path)).collect(),
        determinism: map_determinism(request.determinism),
    }
}
