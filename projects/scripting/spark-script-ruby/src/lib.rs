//! Oaks `oak-ruby` Builder → 公共 IR → `spark-vm` 字节码。
//!
//! 解析只走 [`RubyBuilder`]。优先经 `spark-script-ir`；子集不覆盖时回退旧 lowering。

mod compile;
mod lower;

use compile::compile_root;
use lower::lower_root_to_hir;

use oak_core::{Builder, SourceText};
use oak_ruby::{RubyBuilder, RubyLanguage, RubyRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs};
use spark_script_ir::{emit_module_with_host, lower_module, HostEmitMode};
use spark_vm::Module;

pub use oak_ruby::RubyRoot as ParsedRoot;

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

/// 源码 → [`Module`]。
///
/// 优先走 HIR→MIR→字节码；子集不覆盖则回退旧路径。
pub fn compile(source: &str, natives: &[&str]) -> Result<Module, RubyScriptError> {
    let root = parse(source)?;
    match lower_root_to_hir(&root, natives) {
        Ok(hir) => {
            let mir = lower_module(&hir).map_err(RubyScriptError::compile_opaque)?;
            let host = if natives.is_empty() {
                HostEmitMode::CallNativeByName
            } else {
                HostEmitMode::CallHostSlots(natives)
            };
            emit_module_with_host(&mir, host).map_err(RubyScriptError::compile_opaque)
        }
        Err(_) => compile_root(&root, natives).map_err(RubyScriptError::compile_opaque),
    }
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
        let module = compile("return 40 + 2", &[]).unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
    }

    #[test]
    fn arithmetic_via_ir_has_no_call_native() {
        let module = compile("return 40 + 2", &[]).unwrap();
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
            "#,
            &[],
        )
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
            "#,
            &[],
        )
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
            "#,
            &[],
        )
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
        let module = compile("return ping(7)", &["ping"]).unwrap();
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
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
    }

    #[test]
    fn receiver_call_and_hex() {
        let module = compile(
            r#"
            return 0x2A
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));

        let module = compile(
            r#"
            def Foo_bar
              return 7
            end
            return Foo.bar()
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(7.0));
    }

    #[test]
    fn class_new_ivar_and_global() {
        let module = compile(
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
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(1.0));
    }

    #[test]
    fn each_loop_and_range() {
        let module = compile(
            r#"
            xs = [10, 20]
            s = 0
            xs.each{|v|
              s = s + v
            }
            return s
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(30.0));

        let module = compile(
            r#"
            s = 0
            for i in 1..3
              s = s + i
            end
            return s
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(6.0));
    }

    #[test]
    fn break_on_scene_nil() {
        let module = compile(
            r#"
            n = 0
            while n < 5
              tick()
              n = n + 1
            end
            return n
            "#,
            &["tick"],
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
    fn graphics_update_is_call_native() {
        let module = compile(
            r#"
            i = 0
            while i < 3
              Graphics.update
              i = i + 1
            end
            return i
            "#,
            &["Graphics_update"],
        )
        .unwrap();
        assert!(
            module.functions[module.entry]
                .strings
                .iter()
                .any(|s| s == "Graphics_update"),
            "strings={:?}",
            module.functions[module.entry].strings
        );
        let frames = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let mut vm = Vm::new(module);
        {
            let frames = frames.clone();
            vm.register_native("Graphics_update", move |_ctx, _| {
                frames.set(frames.get() + 1);
                Ok(Value::Null)
            });
        }
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(3.0));
        assert_eq!(frames.get(), 3);
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
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(module);
        vm.step_limit = 50_000_000;
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(6001.0));
    }

    #[test]
    fn default_param_stops_recursive_new() {
        let module = compile(
            r#"
            class Game_Variables
              def initialize(base=false)
                @data = []
                @plus = Game_Variables.new(true) unless base
              end
            end
            $v = Game_Variables.new
            return 1
            "#,
            &[],
        )
        .unwrap();
        let init = module
            .functions
            .iter()
            .find(|f| f.name == "Game_Variables_initialize")
            .expect("initialize");
        assert_eq!(init.arity, 2, "self + base");
        let mut vm = Vm::new(module);
        vm.step_limit = 100_000;
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(1.0));
    }
}
