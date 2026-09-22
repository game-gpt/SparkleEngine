//! 脚本 System 描述符：进入与 Rust System 同一调度图前的声明契约。
//!
//! 串行模式下已强制：before/after 拓扑序、同 phase 组件访问冲突检测、
//! `Exclusive` 不得与其它同 phase System 并存、按 `query_archetypes` 安装受限查询视图。
//! 并行批次仍后续接入。
//! 同一 [`crate::ScriptDomain`] 的执行默认视为串行资源。

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use spark_script::{DeterminismClass, HostPhase};

/// 脚本 System 的并行策略（ECS 读写集不足以证明可并行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScriptParallelism {
    /// 与同域其它脚本串行（默认）。
    #[default]
    SerialDomain,
    /// 与一切脚本 System 互斥。
    Exclusive,
}

/// 组件访问声明（名字在链接期解析为稳定槽位；热路径禁止再字符串查找）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentAccess {
    /// 组件逻辑名。
    pub component: Arc<str>,
    /// `true` = 写；`false` = 只读。
    pub write: bool,
}

impl ComponentAccess {
    /// 只读访问声明。
    pub fn read(component: impl Into<Arc<str>>) -> Self {
        Self { component: component.into(), write: false }
    }

    /// 写访问声明。
    pub fn write(component: impl Into<Arc<str>>) -> Self {
        Self { component: component.into(), write: true }
    }
}

/// 脚本 System 描述符。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSystemDescriptor {
    /// 所属模组 id。
    pub mod_id: Arc<str>,
    /// System 逻辑名（同模组内唯一；调度图键的一部分）。
    pub name: Arc<str>,
    /// 入口导出函数名（如 `fixed_update` / 自定义 micro）。
    pub entry: Arc<str>,
    /// 所属宿主生命周期阶段。
    pub phase: HostPhase,
    /// 组件读写声明（门禁与冲突检测依据）。
    pub access: Vec<ComponentAccess>,
    /// 查询可见的脚本原型名；空 = 不按原型过滤（仍受读写集门禁）。
    pub query_archetypes: Vec<Arc<str>>,
    /// 必须排在本 System **之前** 的其它 System 名 / 图键。
    pub before: Vec<Arc<str>>,
    /// 必须排在本 System **之后** 的前置依赖名 / 图键。
    pub after: Vec<Arc<str>>,
    /// 确定性等级（约束可调用的宿主导入）。
    pub determinism: DeterminismClass,
    /// 并行 / 互斥策略。
    pub parallelism: ScriptParallelism,
}

impl ScriptSystemDescriptor {
    /// 最小描述符：默认定确定性、同域串行、无访问/序约束。
    pub fn new(mod_id: impl Into<Arc<str>>, name: impl Into<Arc<str>>, entry: impl Into<Arc<str>>, phase: HostPhase) -> Self {
        Self {
            mod_id: mod_id.into(),
            name: name.into(),
            entry: entry.into(),
            phase,
            access: Vec::new(),
            query_archetypes: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            determinism: DeterminismClass::Deterministic,
            parallelism: ScriptParallelism::SerialDomain,
        }
    }

    /// 追加只读组件访问。
    pub fn read(mut self, component: impl Into<Arc<str>>) -> Self {
        self.access.push(ComponentAccess::read(component));
        self
    }

    /// 追加写组件访问。
    pub fn write(mut self, component: impl Into<Arc<str>>) -> Self {
        self.access.push(ComponentAccess::write(component));
        self
    }

    /// 限制 `query_*` 可见的脚本原型。
    pub fn query_archetype(mut self, archetype: impl Into<Arc<str>>) -> Self {
        self.query_archetypes.push(archetype.into());
        self
    }

    /// 声明本 System 须排在目标 **之前**。
    pub fn before(mut self, name: impl Into<Arc<str>>) -> Self {
        self.before.push(name.into());
        self
    }

    /// 声明本 System 须排在目标 **之后**。
    pub fn after(mut self, name: impl Into<Arc<str>>) -> Self {
        self.after.push(name.into());
        self
    }

    /// 覆盖确定性等级。
    pub fn determinism(mut self, class: DeterminismClass) -> Self {
        self.determinism = class;
        self
    }

    /// 覆盖并行策略。
    pub fn parallelism(mut self, policy: ScriptParallelism) -> Self {
        self.parallelism = policy;
        self
    }

    /// 调度图节点键：`mod_id/name`。
    pub fn graph_key(&self) -> String {
        format!("{}/{}", self.mod_id, self.name)
    }
}

/// System 登记 / 调度声明错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptSystemError {
    /// before/after 图存在环。
    Cycle {
        /// 诊断细节（含 phase 等）。
        detail: String,
    },
    /// 同 phase 两 System 对同一组件声明冲突的写/读写。
    AccessConflict {
        /// 冲突双方与组件名。
        detail: String,
    },
    /// `Exclusive` 与同 phase 其它 System 并存，或存在多个 Exclusive。
    ExclusiveConflict {
        /// 冲突说明。
        detail: String,
    },
    /// before/after 目标无法唯一解析。
    UnknownOrderTarget {
        /// 无法解析的目标名。
        detail: String,
    },
}

impl std::fmt::Display for ScriptSystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cycle { detail } => write!(f, "spark.engine.script_system.cycle:{detail}"),
            Self::AccessConflict { detail } => {
                write!(f, "spark.engine.script_system.access_conflict:{detail}")
            }
            Self::ExclusiveConflict { detail } => {
                write!(f, "spark.engine.script_system.exclusive_conflict:{detail}")
            }
            Self::UnknownOrderTarget { detail } => {
                write!(f, "spark.engine.script_system.unknown_order:{detail}")
            }
        }
    }
}

impl std::error::Error for ScriptSystemError {}

/// 引擎侧脚本 System 登记表。
#[derive(Debug, Default, Clone)]
pub struct ScriptSystemRegistry {
    systems: Vec<ScriptSystemDescriptor>,
}

impl ScriptSystemRegistry {
    /// 空登记表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 已登记 System 数。
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// 是否尚无登记。
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// 登记或覆盖同 `mod_id`+`name` 的描述符（不立即校验）。
    pub fn register(&mut self, desc: ScriptSystemDescriptor) {
        if let Some(existing) = self.systems.iter_mut().find(|s| s.mod_id == desc.mod_id && s.name == desc.name) {
            *existing = desc;
        }
        else {
            self.systems.push(desc);
        }
    }

    /// 登记并立即校验当前全表声明（环 / 冲突）。
    pub fn register_checked(&mut self, desc: ScriptSystemDescriptor) -> Result<(), ScriptSystemError> {
        self.register(desc);
        self.validate_all()
    }

    /// 移除某模组的全部 System 描述符。
    pub fn remove_mod(&mut self, mod_id: &str) {
        self.systems.retain(|s| s.mod_id.as_ref() != mod_id);
    }

    /// 只读切片访问全部描述符。
    pub fn systems(&self) -> &[ScriptSystemDescriptor] {
        &self.systems
    }

    /// 迭代属于 `phase`（或 `Any`）的描述符。
    pub fn for_phase(&self, phase: HostPhase) -> impl Iterator<Item = &ScriptSystemDescriptor> {
        self.systems.iter().filter(move |s| s.phase == phase || s.phase == HostPhase::Any)
    }

    /// 校验全表：Exclusive、访问冲突、before/after 环。
    pub fn validate_all(&self) -> Result<(), ScriptSystemError> {
        let phases: HashSet<HostPhase> = self.systems.iter().map(|s| s.phase).collect();
        for phase in phases {
            let _ = self.ordered_for_phase(phase)?;
        }
        Ok(())
    }

    /// 按 before/after 拓扑序返回某一 phase 的 System；并检查访问冲突。
    pub fn ordered_for_phase(&self, phase: HostPhase) -> Result<Vec<&ScriptSystemDescriptor>, ScriptSystemError> {
        let jobs: Vec<&ScriptSystemDescriptor> = self.for_phase(phase).collect();
        if jobs.is_empty() {
            return Ok(Vec::new());
        }

        let exclusive: Vec<_> = jobs.iter().filter(|s| s.parallelism == ScriptParallelism::Exclusive).collect();
        if exclusive.len() > 1 {
            return Err(ScriptSystemError::ExclusiveConflict { detail: format!("phase={phase:?} exclusive_count={}", exclusive.len()) });
        }
        if exclusive.len() == 1 && jobs.len() > 1 {
            return Err(ScriptSystemError::ExclusiveConflict {
                detail: format!("phase={phase:?} exclusive={} peers={}", exclusive[0].graph_key(), jobs.len() - 1),
            });
        }

        check_access_conflicts(&jobs)?;

        // 图：edge A→B 表示 A 必须在 B 之前。
        let keys: Vec<String> = jobs.iter().map(|s| s.graph_key()).collect();
        let key_set: HashSet<&str> = keys.iter().map(|s| s.as_str()).collect();
        let index: HashMap<&str, usize> = keys.iter().enumerate().map(|(i, k)| (k.as_str(), i)).collect();

        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); jobs.len()];
        let mut indeg = vec![0usize; jobs.len()];

        for (i, sys) in jobs.iter().enumerate() {
            for after_name in &sys.after {
                let pred = resolve_order_target(&jobs, after_name.as_ref(), &key_set)?;
                let Some(&p) = index.get(pred.as_str())
                else {
                    continue;
                };
                adj[p].push(i);
                indeg[i] += 1;
            }
            for before_name in &sys.before {
                let succ = resolve_order_target(&jobs, before_name.as_ref(), &key_set)?;
                let Some(&s) = index.get(succ.as_str())
                else {
                    continue;
                };
                adj[i].push(s);
                indeg[s] += 1;
            }
        }

        let mut queue: VecDeque<usize> = indeg.iter().enumerate().filter_map(|(i, d)| (*d == 0).then_some(i)).collect();
        let mut ordered = Vec::with_capacity(jobs.len());
        while let Some(i) = queue.pop_front() {
            ordered.push(jobs[i]);
            for &n in &adj[i] {
                indeg[n] -= 1;
                if indeg[n] == 0 {
                    queue.push_back(n);
                }
            }
        }
        if ordered.len() != jobs.len() {
            return Err(ScriptSystemError::Cycle { detail: format!("phase={phase:?}") });
        }
        Ok(ordered)
    }

    /// 由领域生命周期导出生成默认 System（每导出一条）。
    pub fn register_lifecycle_exports(&mut self, mod_id: impl Into<Arc<str>>, exports: &[Arc<str>]) {
        let mod_id = mod_id.into();
        for name in exports {
            let Some(phase) = lifecycle_phase(name.as_ref())
            else {
                continue;
            };
            self.register(ScriptSystemDescriptor::new(Arc::clone(&mod_id), Arc::clone(name), Arc::clone(name), phase));
        }
    }
}

fn resolve_order_target(jobs: &[&ScriptSystemDescriptor], target: &str, key_set: &HashSet<&str>) -> Result<String, ScriptSystemError> {
    if key_set.contains(target) {
        return Ok(target.to_string());
    }
    let matches: Vec<_> = jobs.iter().filter(|s| s.name.as_ref() == target || s.entry.as_ref() == target).map(|s| s.graph_key()).collect();
    match matches.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(ScriptSystemError::UnknownOrderTarget { detail: target.into() }),
        _ => Err(ScriptSystemError::UnknownOrderTarget { detail: format!("ambiguous:{target}") }),
    }
}

fn check_access_conflicts(jobs: &[&ScriptSystemDescriptor]) -> Result<(), ScriptSystemError> {
    // 串行执行仍要求声明互不矛盾：同一组件不得被两个 System 同时声明 write，
    // 或 write 与另一 read（声明层冲突——真正并行前的契约检查）。
    for (i, a) in jobs.iter().enumerate() {
        for b in jobs.iter().skip(i + 1) {
            for aa in &a.access {
                for ba in &b.access {
                    if aa.component != ba.component {
                        continue;
                    }
                    if aa.write || ba.write {
                        return Err(ScriptSystemError::AccessConflict {
                            detail: format!("{} vs {} on {}", a.graph_key(), b.graph_key(), aa.component),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn lifecycle_phase(name: &str) -> Option<HostPhase> {
    match name {
        "on_load" => Some(HostPhase::OnLoad),
        "on_start" => Some(HostPhase::OnStart),
        "fixed_update" => Some(HostPhase::FixedUpdate),
        "update" => Some(HostPhase::Update),
        "late_update" => Some(HostPhase::LateUpdate),
        "render_prepare" => Some(HostPhase::RenderPrepare),
        "on_event" => Some(HostPhase::OnEvent),
        "on_unload" => Some(HostPhase::OnUnload),
        _ => None,
    }
}
