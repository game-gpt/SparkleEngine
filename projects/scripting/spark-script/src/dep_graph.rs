//! 包依赖图：为多目标链接提供拓扑序。
//!
//! 节点是 [`PackageId`]；边是「A 依赖 B」→ 链接时 B 须先于 A。
//! 本模块不解析源码 `import`，由编译会话填入显式依赖。

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use crate::request::PackageId;

/// 依赖图错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepGraphError {
    /// 未知依赖目标。
    UnknownPackage { id: PackageId },
    /// 存在环。
    Cycle { path: Vec<PackageId> },
    /// 空图。
    Empty,
}

impl std::fmt::Display for DepGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownPackage { id } => {
                write!(f, "unknown_package:{}@{}", id.name, id.version)
            }
            Self::Cycle { path } => {
                write!(f, "cycle:")?;
                for (i, p) in path.iter().enumerate() {
                    if i > 0 {
                        write!(f, "->")?;
                    }
                    write!(f, "{}@{}", p.name, p.version)?;
                }
                Ok(())
            }
            Self::Empty => write!(f, "empty_dep_graph"),
        }
    }
}

impl std::error::Error for DepGraphError {}

fn key(id: &PackageId) -> Arc<str> {
    Arc::from(format!("{}@{}", id.name, id.version))
}

/// 单个包节点及其直接依赖（被依赖方）。
#[derive(Debug, Clone)]
pub struct PackageNode {
    pub id: PackageId,
    /// 本包依赖的其它包（须先链接）。
    pub depends_on: Vec<PackageId>,
}

/// 包依赖图。
#[derive(Debug, Default, Clone)]
pub struct PackageDepGraph {
    nodes: HashMap<Arc<str>, PackageNode>,
}

impl PackageDepGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// 插入或替换节点。
    pub fn insert(&mut self, node: PackageNode) {
        self.nodes.insert(key(&node.id), node);
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Kahn 拓扑序：被依赖方在前。
    pub fn topo_order(&self) -> Result<Vec<PackageId>, DepGraphError> {
        if self.nodes.is_empty() {
            return Err(DepGraphError::Empty);
        }
        let mut indegree: HashMap<Arc<str>, usize> = HashMap::new();
        let mut adj: HashMap<Arc<str>, Vec<Arc<str>>> = HashMap::new();
        for id in self.nodes.keys() {
            indegree.insert(Arc::clone(id), 0);
            adj.insert(Arc::clone(id), Vec::new());
        }
        for node in self.nodes.values() {
            let node_key = key(&node.id);
            for dep in &node.depends_on {
                let dep_key = key(dep);
                if !self.nodes.contains_key(&dep_key) {
                    return Err(DepGraphError::UnknownPackage { id: dep.clone() });
                }
                // dep → node：边从依赖指向依赖者
                adj.get_mut(&dep_key).expect("adj").push(Arc::clone(&node_key));
                *indegree.get_mut(&node_key).expect("indegree") += 1;
            }
        }
        let mut queue: VecDeque<Arc<str>> = indegree.iter().filter(|(_, d)| **d == 0).map(|(k, _)| Arc::clone(k)).collect();
        queue.make_contiguous().sort();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(k) = queue.pop_front() {
            let node = self.nodes.get(&k).expect("node");
            order.push(node.id.clone());
            let mut nexts = adj.get(&k).cloned().unwrap_or_default();
            nexts.sort();
            for n in nexts {
                let d = indegree.get_mut(&n).expect("indegree");
                *d -= 1;
                if *d == 0 {
                    let pos = queue.binary_search_by(|x| x.as_ref().cmp(n.as_ref())).unwrap_err();
                    queue.insert(pos, n);
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err(DepGraphError::Cycle { path: find_cycle(&adj, &self.nodes) });
        }
        Ok(order)
    }
}

fn find_cycle(adj: &HashMap<Arc<str>, Vec<Arc<str>>>, nodes: &HashMap<Arc<str>, PackageNode>) -> Vec<PackageId> {
    let mut visiting = HashSet::new();
    let mut path_keys = Vec::new();
    for start in nodes.keys() {
        if dfs_cycle(start, adj, &mut visiting, &mut path_keys) {
            return path_keys.into_iter().filter_map(|k| nodes.get(&k).map(|n| n.id.clone())).collect();
        }
        path_keys.clear();
        visiting.clear();
    }
    nodes.values().take(1).map(|n| n.id.clone()).collect()
}

fn dfs_cycle(node: &Arc<str>, adj: &HashMap<Arc<str>, Vec<Arc<str>>>, visiting: &mut HashSet<Arc<str>>, path: &mut Vec<Arc<str>>) -> bool {
    if !visiting.insert(Arc::clone(node)) {
        path.push(Arc::clone(node));
        return true;
    }
    path.push(Arc::clone(node));
    for n in adj.get(node).into_iter().flatten() {
        if dfs_cycle(n, adj, visiting, path) {
            return true;
        }
    }
    path.pop();
    visiting.remove(node);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str) -> PackageId {
        PackageId::new(name, "1")
    }

    #[test]
    fn topo_libs_before_entry() {
        let mut g = PackageDepGraph::new();
        g.insert(PackageNode { id: pkg("entry"), depends_on: vec![pkg("lib_a"), pkg("lib_b")] });
        g.insert(PackageNode { id: pkg("lib_a"), depends_on: vec![pkg("lib_b")] });
        g.insert(PackageNode { id: pkg("lib_b"), depends_on: vec![] });
        let order = g.topo_order().unwrap();
        let names: Vec<&str> = order.iter().map(|p| p.name.as_ref()).collect();
        assert_eq!(names, vec!["lib_b", "lib_a", "entry"]);
    }

    #[test]
    fn cycle_is_rejected() {
        let mut g = PackageDepGraph::new();
        g.insert(PackageNode { id: pkg("a"), depends_on: vec![pkg("b")] });
        g.insert(PackageNode { id: pkg("b"), depends_on: vec![pkg("a")] });
        assert!(matches!(g.topo_order(), Err(DepGraphError::Cycle { .. })));
    }

    #[test]
    fn unknown_dep_is_rejected() {
        let mut g = PackageDepGraph::new();
        g.insert(PackageNode { id: pkg("a"), depends_on: vec![pkg("missing")] });
        assert!(matches!(g.topo_order(), Err(DepGraphError::UnknownPackage { .. })));
    }
}
