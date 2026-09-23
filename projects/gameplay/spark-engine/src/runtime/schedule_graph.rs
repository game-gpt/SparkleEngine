//! 统一调度图：Rust System 与 Spark Script System 同相位混排。

use std::collections::{HashMap, HashSet, VecDeque};

use spark_script::HostPhase;

use crate::ScriptSystemDescriptor;
use crate::script_system::{ScriptParallelism, ScriptSystemError};

use super::phase::RustPhase;
use super::rust_system::RustSystemMeta;

/// 混排节点（拓扑序输出）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixedNode {
    /// `rust_entries` 切片下标。
    Rust(usize),
    /// `script_entries` 切片下标。
    Script(usize),
}

/// 将 Rust 仿真 / 渲染相位映射到可混排的 Spark Script 相位。
pub fn host_phase_for_rust(phase: RustPhase) -> Option<HostPhase> {
    match phase {
        RustPhase::FixedUpdate => Some(HostPhase::FixedUpdate),
        RustPhase::Update => Some(HostPhase::Update),
        RustPhase::LateUpdate => Some(HostPhase::LateUpdate),
        RustPhase::RenderPrepare => Some(HostPhase::RenderPrepare),
        _ => None,
    }
}

/// 该 Rust 相位是否与 Spark Script 共用统一调度图。
pub fn is_mixed_rust_phase(phase: RustPhase) -> bool {
    host_phase_for_rust(phase).is_some()
}

/// 按 `before` / `after` 拓扑序混排 Rust 与 Script System。
pub fn ordered_mixed_phase(
    rust_systems: &[RustSystemMeta],
    script_systems: &[ScriptSystemDescriptor],
) -> Result<Vec<MixedNode>, ScriptSystemError> {
    if rust_systems.is_empty() && script_systems.is_empty() {
        return Ok(Vec::new());
    }

    check_script_exclusive(script_systems)?;
    check_script_access_conflicts(script_systems)?;

    let rust_len = rust_systems.len();
    let node_count = rust_len + script_systems.len();
    let mut keys = Vec::with_capacity(node_count);
    keys.extend(rust_systems.iter().map(RustSystemMeta::graph_key));
    keys.extend(script_systems.iter().map(ScriptSystemDescriptor::graph_key));
    let key_set: HashSet<&str> = keys.iter().map(|s| s.as_str()).collect();
    let key_to_node: HashMap<&str, MixedNode> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let node = if i < rust_len { MixedNode::Rust(i) } else { MixedNode::Script(i - rust_len) };
            (k.as_str(), node)
        })
        .collect();

    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); node_count];
    let mut indeg = vec![0usize; node_count];

    let mut add_edge = |from: usize, to: usize| {
        if from != to {
            adj[from].push(to);
            indeg[to] += 1;
        }
    };

    let node_index = |node: MixedNode| -> usize {
        match node {
            MixedNode::Rust(i) => i,
            MixedNode::Script(i) => rust_len + i,
        }
    };

    for (ri, meta) in rust_systems.iter().enumerate() {
        for after_name in &meta.after {
            let pred_key = resolve_order_target(after_name, rust_systems, script_systems, &key_set)?;
            let pred = key_to_node.get(pred_key.as_str()).copied().ok_or_else(|| ScriptSystemError::UnknownOrderTarget {
                detail: after_name.to_string(),
            })?;
            add_edge(node_index(pred), ri);
        }
        for before_name in &meta.before {
            let succ_key = resolve_order_target(before_name, rust_systems, script_systems, &key_set)?;
            let succ = key_to_node.get(succ_key.as_str()).copied().ok_or_else(|| ScriptSystemError::UnknownOrderTarget {
                detail: before_name.to_string(),
            })?;
            add_edge(ri, node_index(succ));
        }
    }

    for (si, desc) in script_systems.iter().enumerate() {
        let i = rust_len + si;
        for after_name in &desc.after {
            let pred_key = resolve_order_target(after_name.as_ref(), rust_systems, script_systems, &key_set)?;
            let pred = key_to_node.get(pred_key.as_str()).copied().ok_or_else(|| ScriptSystemError::UnknownOrderTarget {
                detail: after_name.to_string(),
            })?;
            add_edge(node_index(pred), i);
        }
        for before_name in &desc.before {
            let succ_key = resolve_order_target(before_name.as_ref(), rust_systems, script_systems, &key_set)?;
            let succ = key_to_node.get(succ_key.as_str()).copied().ok_or_else(|| ScriptSystemError::UnknownOrderTarget {
                detail: before_name.to_string(),
            })?;
            add_edge(i, node_index(succ));
        }
    }

    check_rust_access_conflicts(rust_systems)?;

    let mut queue: VecDeque<usize> = indeg.iter().enumerate().filter_map(|(i, d)| (*d == 0).then_some(i)).collect();
    let mut ordered_indices = Vec::with_capacity(node_count);
    while let Some(i) = queue.pop_front() {
        ordered_indices.push(i);
        for &n in &adj[i] {
            indeg[n] -= 1;
            if indeg[n] == 0 {
                queue.push_back(n);
            }
        }
    }
    if ordered_indices.len() != node_count {
        return Err(ScriptSystemError::Cycle { detail: "mixed_phase".into() });
    }

    Ok(ordered_indices
        .into_iter()
        .map(|i| if i < rust_len { MixedNode::Rust(i) } else { MixedNode::Script(i - rust_len) })
        .collect())
}

fn check_script_exclusive(script_systems: &[ScriptSystemDescriptor]) -> Result<(), ScriptSystemError> {
    let exclusive: Vec<_> = script_systems.iter().filter(|s| s.parallelism == ScriptParallelism::Exclusive).collect();
    if exclusive.len() > 1 {
        return Err(ScriptSystemError::ExclusiveConflict { detail: format!("exclusive_count={}", exclusive.len()) });
    }
    if exclusive.len() == 1 && script_systems.len() > 1 {
        return Err(ScriptSystemError::ExclusiveConflict {
            detail: format!("exclusive={} peers={}", exclusive[0].graph_key(), script_systems.len() - 1),
        });
    }
    Ok(())
}

fn resolve_order_target(
    target: &str,
    rust_systems: &[RustSystemMeta],
    script_systems: &[ScriptSystemDescriptor],
    key_set: &HashSet<&str>,
) -> Result<String, ScriptSystemError> {
    if key_set.contains(target) {
        return Ok(target.to_string());
    }
    if !target.contains('/') {
        if let Some(meta) = rust_systems.iter().find(|m| m.name == target) {
            return Ok(meta.graph_key());
        }
    }
    let script_matches: Vec<_> = script_systems
        .iter()
        .filter(|s| s.name.as_ref() == target || s.entry.as_ref() == target)
        .map(ScriptSystemDescriptor::graph_key)
        .collect();
    match script_matches.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(ScriptSystemError::UnknownOrderTarget { detail: target.into() }),
        _ => Err(ScriptSystemError::UnknownOrderTarget { detail: format!("ambiguous:{target}") }),
    }
}

fn check_script_access_conflicts(jobs: &[ScriptSystemDescriptor]) -> Result<(), ScriptSystemError> {
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

fn check_rust_access_conflicts(systems: &[RustSystemMeta]) -> Result<(), ScriptSystemError> {
    for (i, a) in systems.iter().enumerate() {
        for b in systems.iter().skip(i + 1) {
            for ta in &a.writes {
                if b.writes.contains(ta) || b.reads.contains(ta) {
                    return Err(ScriptSystemError::AccessConflict {
                        detail: format!("{} vs {} on type_id {:?}", a.graph_key(), b.graph_key(), ta),
                    });
                }
            }
            for tb in &b.writes {
                if a.reads.contains(tb) {
                    return Err(ScriptSystemError::AccessConflict {
                        detail: format!("{} vs {} on type_id {:?}", a.graph_key(), b.graph_key(), tb),
                    });
                }
            }
        }
    }
    Ok(())
}
