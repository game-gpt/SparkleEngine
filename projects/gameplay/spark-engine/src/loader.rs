//! 模组发现与依赖拓扑排序。

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::manifest::ModManifest;
use crate::vfs::ModVfs;
use crate::EngineError;
use spark_script::ScriptEngine;

/// 已加载模组。
pub struct LoadedMod {
    pub manifest: ModManifest,
    pub root: std::path::PathBuf,
    pub vfs: ModVfs,
    pub script: Option<ScriptEngine>,
    pub enabled: bool,
}

pub struct ModLoader;

/// 扫描根目录，返回按依赖排序的清单列表。
pub fn discover_and_order(mods_root: &Path) -> Result<Vec<ModManifest>, EngineError> {
    let mut by_id: HashMap<String, ModManifest> = HashMap::new();
    if !mods_root.exists() {
        return Ok(Vec::new());
    }
    let rd = std::fs::read_dir(mods_root).map_err(|e| {
        EngineError::io(mods_root.display().to_string(), e.to_string())
    })?;
    for ent in rd.flatten() {
        let p = ent.path();
        if !p.is_dir() {
            continue;
        }
        let von = p.join("mod.von");
        if !von.is_file() {
            continue;
        }
        let m = ModManifest::from_path(&von)?;
        if by_id.contains_key(&m.id) {
            return Err(EngineError::DuplicateMod { id: m.id });
        }
        by_id.insert(m.id.clone(), m);
    }

    // 校验依赖存在
    for m in by_id.values() {
        for dep in &m.dependencies {
            if !by_id.contains_key(dep) {
                return Err(EngineError::MissingDep {
                    mod_id: m.id.clone(),
                    dep: dep.clone(),
                });
            }
        }
    }

    topological_sort(by_id)
}

fn topological_sort(
    mut by_id: HashMap<String, ModManifest>,
) -> Result<Vec<ModManifest>, EngineError> {
    let ids: Vec<String> = by_id.keys().cloned().collect();
    let mut indeg: HashMap<String, usize> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for id in &ids {
        indeg.entry(id.clone()).or_insert(0);
    }
    for m in by_id.values() {
        for dep in &m.dependencies {
            *indeg.entry(m.id.clone()).or_insert(0) += 1;
            dependents
                .entry(dep.clone())
                .or_default()
                .push(m.id.clone());
        }
    }
    let mut q: VecDeque<String> = indeg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(k, _)| k.clone())
        .collect();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    while let Some(id) = q.pop_front() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(m) = by_id.remove(&id) {
            out.push(m);
        }
        if let Some(nexts) = dependents.get(&id) {
            for n in nexts {
                if let Some(d) = indeg.get_mut(n) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        q.push_back(n.clone());
                    }
                }
            }
        }
    }
    if out.len() != ids.len() {
        let left: Vec<_> = ids.into_iter().filter(|i| !seen.contains(i)).collect();
        return Err(EngineError::CyclicDeps {
            mods: left.join(", "),
        });
    }
    Ok(out)
}
