//! 自 `src/domain.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

use spark_script::{HostFunction, HostFunctionId, ScriptCompiler, ScriptLanguage};
use spark_vm::StdHost;

use spark_script::HostSchema;
#[test]
fn domain_loads_image_and_calls_lifecycle() {
    let mut host = HostSchema::new(1);
    host.insert(HostFunction::new(HostFunctionId::new("host", "noop", 1)));
    let source = r#"
        micro on_start() {
            return 42
        }
        return 0
        "#;
    let mut compiler = ScriptCompiler::new();
    let package = compiler.compile_source(ScriptLanguage::Valkyrie, source, &host).unwrap();
    assert!(package.image.lifecycle_exports.iter().any(|n| n.as_ref() == "on_start"));
    let mut domain = ScriptDomain::from_image("test.mod", &package.image, &host, ScriptBudget::default()).unwrap();
    let mut hooks = StdHost;
    let v = domain.call_lifecycle("on_start", &[], &mut hooks).unwrap().unwrap();
    assert_eq!(v.as_number(), Some(42.0));
    assert!(domain.call_lifecycle("on_unload", &[], &mut hooks).unwrap().is_none());
    assert_eq!(domain.runtime.vm.step_limit, ScriptBudget::default().instruction_limit);
}

#[test]
fn custom_budget_sets_vm_limits() {
    let host = HostSchema::new(1);
    let mut compiler = ScriptCompiler::new();
    let package = compiler.compile_source(ScriptLanguage::Valkyrie, "return 1", &host).unwrap();
    let budget = ScriptBudget { instruction_limit: 1234, host_call_limit: 56, allocation_limit: 78, call_depth_limit: 9 };
    let domain = ScriptDomain::from_image("budget.mod", &package.image, &host, budget).unwrap();
    assert_eq!(domain.runtime.vm.step_limit, 1234);
    assert_eq!(domain.runtime.vm.host_call_limit, 56);
    assert_eq!(domain.runtime.vm.allocation_limit, 78);
    assert_eq!(domain.runtime.vm.call_depth_limit, 9);
}

#[test]
fn domain_command_buffer_drains() {
    let host = HostSchema::new(1);
    let mut compiler = ScriptCompiler::new();
    let package = compiler.compile_source(ScriptLanguage::Valkyrie, "return 1", &host).unwrap();
    let mut domain = ScriptDomain::from_image("buf.mod", &package.image, &host, ScriptBudget::default()).unwrap();
    domain.command_buffer.borrow_mut().spawn("rock");
    let cmds = domain.drain_commands();
    assert_eq!(cmds.len(), 1);
    assert!(domain.command_buffer.borrow().is_empty());
}

#[test]
fn domain_dispatches_named_event_export() {
    let source = r#"
        micro ping() {
            return 7
        }
        return 0
        "#;
    let host = HostSchema::new(1);
    let mut compiler = ScriptCompiler::new();
    let package = compiler.compile_source(ScriptLanguage::Valkyrie, source, &host).unwrap();
    let mut domain = ScriptDomain::from_image("ev.mod", &package.image, &host, ScriptBudget::default()).unwrap();
    domain.enqueue_event("ping", vec![]);
    let mut hooks = StdHost;
    domain.dispatch_events(&mut hooks).unwrap();
    assert!(domain.event_inbox.is_empty());
}
