//! 编译期与运行期共用的结构化宿主 ABI。
//!
//! 旧 [`NativeRegistry`] 仅含函数名与简单类型路径，不足以支撑效果检查、
//! 能力校验与槽位绑定。新代码应使用本模块的 [`HostSchema`]；
//! [`NativeRegistry`] 可通过 [`HostSchema::from_native_registry`] 升级。

use std::sync::Arc;

use spark_script_valkyrie::{NativeParam, NativeRegistry, NativeSignature, TypeRef};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeterminismClass {
    Deterministic,
    FrameLocal,
    Nondeterministic,
}

/// 允许执行的生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostPhase {
    OnLoad,
    OnStart,
    FixedUpdate,
    Update,
    LateUpdate,
    RenderPrepare,
    OnEvent,
    OnUnload,
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

    /// 从旧 [`NativeSignature`] 升级；命名空间默认 `host`，ABI 版本 `1`。
    pub fn from_native_signature(sig: &NativeSignature) -> Self {
        let mut f = Self::new(HostFunctionId::new("host", Arc::clone(&sig.name), 1));
        f.params = sig.params.clone();
        f.return_ty = sig.return_ty.clone();
        f.docs = sig.docs.clone();
        f
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

    /// 兼容旧编译入口的短名（不含命名空间）。
    pub fn short_name(&self) -> &str {
        self.id.name.as_ref()
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
        Self {
            abi_version,
            functions: Vec::new(),
        }
    }

    pub fn insert(&mut self, func: HostFunction) {
        if let Some(existing) = self
            .functions
            .iter_mut()
            .find(|f| f.id == func.id)
        {
            *existing = func;
        } else {
            self.functions.push(func);
        }
    }

    pub fn get(&self, id: &HostFunctionId) -> Option<&HostFunction> {
        self.functions.iter().find(|f| f.id == *id)
    }

    pub fn get_by_short_name(&self, name: &str) -> Option<&HostFunction> {
        self.functions.iter().find(|f| f.id.name.as_ref() == name)
    }

    /// 链接后的稳定槽位下标（按插入顺序）。
    pub fn slot_of(&self, id: &HostFunctionId) -> Option<u32> {
        self.functions
            .iter()
            .position(|f| f.id == *id)
            .map(|i| i as u32)
    }

    pub fn short_names(&self) -> Vec<&str> {
        self.functions.iter().map(|f| f.short_name()).collect()
    }

    /// 由旧 [`NativeRegistry`] 升级。
    pub fn from_native_registry(reg: &NativeRegistry) -> Self {
        let mut schema = Self::new(1);
        for sig in &reg.signatures {
            schema.insert(HostFunction::from_native_signature(sig));
        }
        schema
    }

    /// 导出兼容旧入口的 [`NativeRegistry`]（丢失效果 / 能力等元数据）。
    pub fn to_native_registry(&self) -> NativeRegistry {
        let mut reg = NativeRegistry::new();
        for func in &self.functions {
            let mut sig = NativeSignature::new(Arc::clone(&func.id.name));
            sig.params = func.params.clone();
            sig.return_ty = func.return_ty.clone();
            sig.docs = func.docs.clone();
            reg.insert(sig);
        }
        reg
    }

    /// schema 内容指纹（缓存键 / 装载校验用）。
    pub fn content_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
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

impl From<&NativeRegistry> for HostSchema {
    fn from(value: &NativeRegistry) -> Self {
        Self::from_native_registry(value)
    }
}

impl From<&HostSchema> for NativeRegistry {
    fn from(value: &HostSchema) -> Self {
        value.to_native_registry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_slots_and_hash_are_stable() {
        let mut schema = HostSchema::new(1);
        schema.insert(
            HostFunction::new(HostFunctionId::new("spark.ecs", "spawn", 1))
                .param(NativeParam::new("archetype", "ArchetypeHandle"))
                .returns("PendingEntity")
                .effect(HostEffect::SpawnEntity)
                .capability("ecs.command")
                .phases([HostPhase::FixedUpdate, HostPhase::Update]),
        );
        schema.insert(
            HostFunction::new(HostFunctionId::new("spark.log", "print", 1))
                .param(NativeParam::new("msg", "String"))
                .returns("Null")
                .effect(HostEffect::Nondeterministic)
                .determinism(DeterminismClass::Nondeterministic),
        );

        let spawn_id = HostFunctionId::new("spark.ecs", "spawn", 1);
        assert_eq!(schema.slot_of(&spawn_id), Some(0));
        assert_eq!(schema.short_names(), vec!["spawn", "print"]);
        let h1 = schema.content_hash();
        let h2 = schema.content_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn native_registry_roundtrip_preserves_names() {
        let mut reg = NativeRegistry::new();
        reg.insert(
            NativeSignature::new("ping")
                .param(NativeParam::new("n", "Number"))
                .returns("Number"),
        );
        let schema = HostSchema::from_native_registry(&reg);
        assert!(schema.get_by_short_name("ping").is_some());
        let back = schema.to_native_registry();
        assert_eq!(back.names(), vec!["ping"]);
    }
}
