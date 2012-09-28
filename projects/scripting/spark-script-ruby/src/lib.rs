//! Oaks `oak-ruby` Builder → 公共 IR → `spark-vm` 字节码。
//!
//! 解析只走 [`RubyBuilder`]。正式编译只经 `spark-script-ir`；不支持的构造必须报错。

mod lower;

use lower::lower_root_to_hir;

use oak_core::{Builder, SourceText};
use oak_ruby::{RubyBuilder, RubyLanguage, RubyRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs};
use spark_script_ir::{emit_module_with_host, lower_module, HostEmitMode};
use spark_vm::Module;

pub use oak_ruby::RubyRoot as ParsedRoot;
pub use spark_script_ir::HostBindTable;

#[derive(Debug)]
pub enum RubyScriptError {
    Parse { args: ErrorArgs },
    Compile { args: ErrorArgs },
}

impl RubyScriptError {
    pub fn parse_failed(diag_count: u64) -> Self {
        Self::Parse {
            args: ErrorArgs::new()
                .with("reason", ErrorArg::String(std::sync::Arc::from("parse_failed")))
                .with("diagnostics", ErrorArg::Unsigned(diag_count)),
        }
    }

    pub fn compile_reason(reason: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Compile {
            args: ErrorArgs::new().with("reason", ErrorArg::String(reason.into())),
        }
    }

    pub fn parse_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::Parse {
            args: ErrorArgs::new().with("reason", ErrorArg::String(detail.into())),
        }
    }

    pub fn compile_opaque(detail: impl Into<std::sync::Arc<str>>) -> Self {
        Self::compile_reason(detail)
    }
}

impl std::fmt::Display for RubyScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { args } => {
                write!(f, "spark.script.ruby.parse")?;
                if let Some(ErrorArg::Unsigned(n)) = args.get("diagnostics") {
                    write!(f, ":diagnostics={n}")?;
                }
                if let Some(ErrorArg::String(s)) = args.get("reason") {
                    write!(f, ":{s}")?;
                }
                Ok(())
            }
            Self::Compile { args } => {
                write!(f, "spark.script.ruby.compile")?;
                if let Some(ErrorArg::String(s)) = args.get("reason") {
                    write!(f, ":{s}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for RubyScriptError {}

/// 无宿主绑定的源码 → [`Module`]。有宿主时请用 [`compile_with_binds`]。
pub fn compile(source: &str) -> Result<Module, RubyScriptError> {
    compile_with_binds(source, &HostBindTable::new())
}

/// 带完整宿主绑定表的编译入口。
pub fn compile_with_binds(
    source: &str,
    hosts: &HostBindTable,
) -> Result<Module, RubyScriptError> {
    let root = parse(source)?;
    let hir = lower_root_to_hir(&root, hosts).map_err(RubyScriptError::compile_opaque)?;
    let mir = lower_module(&hir).map_err(RubyScriptError::compile_opaque)?;
    let mode = if hosts.is_empty() {
        HostEmitMode::NoHost
    } else {
        HostEmitMode::Bound(hosts)
    };
    emit_module_with_host(&mir, mode).map_err(RubyScriptError::compile_opaque)
}

/// 解析为 Oaks AST。
pub fn parse(source: &str) -> Result<RubyRoot, RubyScriptError> {
    let language = RubyLanguage::default();
    let builder = RubyBuilder::new(&language);
    let text = SourceText::new(source);
    let mut session = oak_core::ParseSession::<RubyLanguage>::default();
    let out = builder.build(&text, &[], &mut session);
    match out.result {
        Ok(root) => Ok(root),
        Err(_err) => Err(RubyScriptError::parse_failed(out.diagnostics.len() as u64)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    use spark_gc::Value;
    use spark_vm::{Op, StdHost, Vm};

    #[test]
    fn arithmetic_main() {
        let module = compile("return 40 + 2").unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
    }

    #[test]
    fn arithmetic_via_ir_has_no_call_native() {
        let module = compile("return 40 + 2").unwrap();
        assert!(module.functions.iter().all(|f| {
            !f.code
                .iter()
                .any(|&b| b == Op::CallNative as u8 || b == Op::CallHost as u8)
        }));
    }

    #[test]
    fn local_and_if_via_ir() {
        let module = compile(
            r#"
            x = 1
            if x < 2
              return 42
            else
              return 0
            end
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
    }

    #[test]
    fn while_via_ir() {
        let module = compile(
            r#"
            n = 0
            while n < 3
              n = n + 1
            end
            return n
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(3.0));
    }

    #[test]
    fn until_via_ir() {
        let module = compile(
            r#"
            n = 0
            until n >= 3
              n = n + 1
            end
            return n
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(3.0));
    }

    #[test]
    fn and_or_via_ir() {
        let module = compile(
            r#"
            a = false
            b = 7
            if a && b
              return 1
            end
            if a || b
              return 42
            end
            return 0
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
    }

    #[test]
    fn for_range_via_ir() {
        let module = compile(
            r#"
            n = 0
            for i in 1..3
              n = n + i
            end
            return n
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(6.0));
    }

    #[test]
    fn break_via_ir() {
        let module = compile(
            r#"
            n = 0
            while true
              n = n + 1
              if n >= 3
                break
              end
            end
            return n
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(3.0));
    }

    #[test]
    fn method_via_ir() {
        let module = compile(
            r#"
            def add(a, b)
              return a + b
            end
            return add(40, 2)
            "#)
        .unwrap();
        assert!(
            module.functions.iter().any(|f| f.name == "add"),
            "expected IR method proto"
        );
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
    }

    #[test]
    fn host_call_via_ir() {
        let module = compile_with_binds(
            "return ping(7)",
            &HostBindTable::from_ids([spark_script_ir::HostId::new("host", "ping", 1)]).unwrap(),
        )
        .unwrap();
        assert!(module
            .functions
            .iter()
            .any(|f| f.code.iter().any(|&b| b == Op::CallHost as u8)));
        let mut vm = Vm::new(module);
        vm.prepare_host_slots(["ping"]);
        vm.register_native("ping", |_ctx, args| {
            let n = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);
            Ok(Value::Number(n + 1.0))
        });
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(8.0));
    }

    #[test]
    fn method_call() {
        let module = compile(
            r#"
            def add(a, b)
              return a + b
            end
            return add(40, 2)
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
    }

    #[test]
    fn hex_literal_via_ir() {
        let module = compile(
            r#"
            return 0x2A
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
    }

    #[test]
    fn receiver_call_is_unsupported() {
        let err = compile(
            r#"
            def Foo_bar
              return 7
            end
            return Foo.bar()
            "#)
        .unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("ir_unsupported") || msg.contains("unsupported"),
            "{msg}"
        );
    }

    #[test]
    fn class_is_unsupported() {
        let err = compile(
            r#"
            class Counter
              def initialize
                @n = 0
              end
              def bump
                @n = @n + 1
                return @n
              end
            end
            $c = Counter.new()
            return $c.bump()
            "#)
        .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
    }

    #[test]
    fn each_block_is_unsupported() {
        let err = compile(
            r#"
            xs = [10, 20]
            s = 0
            xs.each{|v|
              s = s + v
            }
            return s
            "#)
        .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
    }

    #[test]
    fn for_range_still_works_without_each() {
        let module = compile(
            r#"
            s = 0
            for i in 1..3
              s = s + i
            end
            return s
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(6.0));
    }

    #[test]
    fn break_on_scene_nil() {
        let module = compile_with_binds(
            r#"
            n = 0
            while n < 5
              tick()
              n = n + 1
            end
            return n
            "#,
            &HostBindTable::from_ids([spark_script_ir::HostId::new("host", "tick", 1)]).unwrap(),
        )
        .unwrap();
        let mut vm = Vm::new(module);
        vm.prepare_host_slots(["tick"]);
        let frames = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let frames2 = frames.clone();
        vm.register_native("tick", move |_ctx, _args| {
            frames2.set(frames2.get() + 1);
            Ok(Value::Null)
        });
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(5.0));
        assert_eq!(frames.get(), 5);
    }

    #[test]
    fn qualified_host_call_is_unsupported_without_ir() {
        // `Graphics.update` 需接收者调用；公共 IR 未覆盖前必须明确失败。
        let err = compile_with_binds(
            r#"
            i = 0
            while i < 3
              Graphics.update
              i = i + 1
            end
            return i
            "#,
            &HostBindTable::from_ids([spark_script_ir::HostId::new(
                "host",
                "Graphics_update",
                1,
            )])
            .unwrap(),
        )
        .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
    }

    #[test]
    fn for_large_range_finishes() {
        let module = compile(
            r#"
            n = 0
            for i in 0..6000
              n = n + 1
            end
            return n
            "#)
        .unwrap();
        let mut vm = Vm::new(module);
        vm.step_limit = 50_000_000;
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(6001.0));
    }

    #[test]
    fn default_param_class_is_unsupported() {
        let err = compile(
            r#"
            class Game_Variables
              def initialize(base=false)
                @data = []
                @plus = Game_Variables.new(true) unless base
              end
            end
            $v = Game_Variables.new
            return 1
            "#)
        .unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("ir_unsupported") || msg.contains("unsupported"), "{msg}");
    }
}
