//! 自 `src/graph.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_circuit::*;

#[test]
fn control_path_and_cut() {
    let mut g = CircuitGraph::new();
    let cockpit = g.add_node();
    let cable = g.add_node();
    let engine_a = g.add_node();
    let engine_b = g.add_node();
    g.link(cockpit, cable, Channel::CONTROL).unwrap();
    g.link(cable, engine_a, Channel::CONTROL).unwrap();
    g.link(cable, engine_b, Channel::CONTROL).unwrap();

    assert!(g.can_reach(cockpit, engine_a, Channel::CONTROL).unwrap());
    assert!(g.can_reach(cockpit, engine_b, Channel::CONTROL).unwrap());

    g.unlink(cable, engine_b, Channel::CONTROL).unwrap();
    assert!(g.can_reach(cockpit, engine_a, Channel::CONTROL).unwrap());
    assert!(!g.can_reach(cockpit, engine_b, Channel::CONTROL).unwrap());
}

#[test]
fn multi_source_reachability() {
    let mut g = CircuitGraph::new();
    let a = g.add_node();
    let b = g.add_node();
    let c = g.add_node();
    g.link(a, b, Channel::POWER).unwrap();
    let mask = g.reachable_from_any(&[a, c], Channel::POWER).unwrap();
    assert!(mask[a as usize]);
    assert!(mask[b as usize]);
    assert!(mask[c as usize]);
}

#[test]
fn connected_components_split_on_cut() {
    let mut g = CircuitGraph::new();
    let a = g.add_node();
    let b = g.add_node();
    let c = g.add_node();
    g.link(a, b, Channel::CONTROL).unwrap();
    g.link(b, c, Channel::CONTROL).unwrap();
    assert_eq!(g.connected_components(Channel::CONTROL).iter().max(), Some(&0));
    assert!(g.same_component(a, c, Channel::CONTROL).unwrap());

    g.unlink(b, c, Channel::CONTROL).unwrap();
    let lists = g.component_lists(Channel::CONTROL);
    assert_eq!(lists.len(), 2);
    assert!(!g.same_component(a, c, Channel::CONTROL).unwrap());
}

#[test]
fn power_budget_cut_isolates_load() {
    let mut g = CircuitGraph::new();
    let source = g.add_node();
    let cable = g.add_node();
    let load = g.add_node();
    g.set_power_source(source, 10.0).unwrap();
    g.set_power_sink(load, 4.0).unwrap();
    g.link(source, cable, Channel::POWER).unwrap();
    g.link(cable, load, Channel::POWER).unwrap();
    let ok = g.power_budget(&[source]).unwrap();
    assert!(ok.satisfied());
    assert!((ok.supply - 10.0).abs() < 1e-5);
    assert!((ok.demand - 4.0).abs() < 1e-5);

    g.unlink(cable, load, Channel::POWER).unwrap();
    let cut = g.power_budget(&[source]).unwrap();
    assert!((cut.demand - 0.0).abs() < 1e-5);
    assert!(cut.satisfied());
}
