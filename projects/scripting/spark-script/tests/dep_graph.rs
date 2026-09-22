//! 自 `src/dep_graph.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_script::*;

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
