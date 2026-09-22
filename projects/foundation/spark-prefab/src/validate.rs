//! Prefab 结构与嵌套图校验。

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
};

use spark_asset::AssetRef;

use crate::{
    document::{PREFAB_SCHEMA, PrefabDocument},
    error::PrefabError,
    r#override::{OverridePath, parse_override_path},
};

impl PrefabDocument {
    /// 校验本文件结构（不含跨文件嵌套环）。
    pub fn validate(&self) -> Result<(), PrefabError> {
        if self.schema != PREFAB_SCHEMA {
            return Err(PrefabError::BadSchema { found: self.schema.clone() });
        }
        for id in self.nodes.keys() {
            validate_node_id(id)?;
        }
        if !self.nodes.contains_key(&self.root) {
            return Err(PrefabError::RootMissing { root: self.root.clone() });
        }
        for (id, node) in &self.nodes {
            for child in &node.children {
                if !self.nodes.contains_key(child) {
                    return Err(PrefabError::ChildMissing { parent: id.clone(), child: child.clone() });
                }
            }
        }
        detect_node_cycle(self)?;
        Ok(())
    }

    /// 解析节点路径（`player/body`）为节点 ID 链；须以 `root` 开头且走通 children。
    pub fn resolve_node_path(&self, path: &str) -> Result<Vec<String>, PrefabError> {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() || parts[0] != self.root {
            return Err(PrefabError::OverrideTargetMissing { target: path.into() });
        }
        let mut chain = Vec::new();
        let mut current = parts[0];
        chain.push(current.to_string());
        if !self.nodes.contains_key(current) {
            return Err(PrefabError::OverrideTargetMissing { target: path.into() });
        }
        for part in &parts[1..] {
            let node = self.nodes.get(current).expect("checked");
            if !node.children.iter().any(|c| c == part) {
                return Err(PrefabError::OverrideTargetMissing { target: path.into() });
            }
            current = part;
            chain.push((*part).to_string());
        }
        Ok(chain)
    }

    /// 校验一条覆盖键是否指向本 Prefab 内存在的节点与组件。
    pub fn validate_override_key(&self, key: &str) -> Result<OverridePath, PrefabError> {
        let parsed = parse_override_path(key)?;
        self.resolve_node_path(&parsed.node_path)?;
        let node_id = parsed.node_path.rsplit('/').next().unwrap_or(parsed.node_path.as_str());
        let node = self.nodes.get(node_id).ok_or_else(|| PrefabError::OverrideTargetMissing { target: key.into() })?;
        if !node.components.contains_key(&parsed.component) {
            return Err(PrefabError::OverrideTargetMissing { target: key.into() });
        }
        Ok(parsed)
    }

    /// 收集本文件直接嵌套的 Prefab 路径引用。
    pub fn nested_prefab_paths(&self) -> Vec<String> {
        let mut out = BTreeSet::new();
        for node in self.nodes.values() {
            if let Some(prefab) = &node.prefab {
                out.insert(prefab.path().to_string());
            }
        }
        out.into_iter().collect()
    }

    /// 校验嵌套图无环。`load` 按路径加载依赖 Prefab。
    pub fn validate_nesting<F>(&self, self_path: &str, mut load: F) -> Result<(), PrefabError>
    where
        F: FnMut(&str) -> Result<PrefabDocument, PrefabError>,
    {
        self.validate()?;
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        fn dfs<F>(
            path: &str,
            doc: &PrefabDocument,
            load: &mut F,
            visiting: &mut HashSet<String>,
            visited: &mut HashSet<String>,
        ) -> Result<(), PrefabError>
        where
            F: FnMut(&str) -> Result<PrefabDocument, PrefabError>,
        {
            if visited.contains(path) {
                return Ok(());
            }
            if !visiting.insert(path.to_string()) {
                return Err(PrefabError::PrefabCycle { path: path.into() });
            }
            doc.validate()?;
            for nested in doc.nested_prefab_paths() {
                let child = load(&nested)?;
                dfs(&nested, &child, load, visiting, visited)?;
            }
            visiting.remove(path);
            visited.insert(path.to_string());
            Ok(())
        }
        dfs(self_path, self, &mut load, &mut visiting, &mut visited)
    }
}

fn validate_node_id(id: &str) -> Result<(), PrefabError> {
    if id.is_empty() || id.contains('/') {
        return Err(PrefabError::BadNodeId { node: id.into() });
    }
    Ok(())
}

fn detect_node_cycle(doc: &PrefabDocument) -> Result<(), PrefabError> {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    fn walk(doc: &PrefabDocument, id: &str, visiting: &mut HashSet<String>, visited: &mut HashSet<String>) -> Result<(), PrefabError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.to_string()) {
            return Err(PrefabError::NodeCycle { node: id.into() });
        }
        if let Some(node) = doc.nodes.get(id) {
            for child in &node.children {
                walk(doc, child, visiting, visited)?;
            }
        }
        visiting.remove(id);
        visited.insert(id.to_string());
        Ok(())
    }
    walk(doc, &doc.root, &mut visiting, &mut visited)?;
    Ok(())
}

/// 从磁盘路径加载并做嵌套环检测的便捷入口。
pub fn validate_prefab_file(path: impl AsRef<Path>) -> Result<PrefabDocument, PrefabError> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy().to_string();
    let doc = PrefabDocument::load(path)?;
    let mut cache: HashMap<String, PrefabDocument> = HashMap::new();
    cache.insert(path_str.clone(), doc.clone());
    doc.validate_nesting(&path_str, |p| {
        if let Some(hit) = cache.get(p) {
            return Ok(hit.clone());
        }
        let loaded = PrefabDocument::load(Path::new(p))?;
        cache.insert(p.to_string(), loaded.clone());
        Ok(loaded)
    })?;
    Ok(doc)
}

/// 供测试与工具构造带路径的嵌套 `AssetRef`。
pub fn nested_ref(path: impl Into<String>) -> AssetRef {
    AssetRef::from_path(path)
}
