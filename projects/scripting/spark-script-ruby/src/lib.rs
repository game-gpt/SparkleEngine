//! Oaks `oak-ruby` Builder → `spark-vm` 字节码。
//!
//! 解析只走 [`RubyBuilder`]。本 crate 只做子集字节码下沉。

mod compile;

use compile::compile_root;

use oak_core::{Builder, SourceText};
use oak_ruby::{RubyBuilder, RubyLanguage, RubyRoot};
use spark_diagnostics::{ErrorArg, ErrorArgs};
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
pub fn compile(source: &str, natives: &[&str]) -> Result<Module, RubyScriptError> {
    let root = parse(source)?;
    compile_root(&root, natives).map_err(RubyScriptError::compile_opaque)
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
    use spark_vm::{StdHost, Vm};

    #[test]
    fn arithmetic_main() {
        let module = compile("return 40 + 2", &[]).unwrap();
        let mut vm = Vm::new(module);
        let value = vm.run(&mut StdHost).unwrap();
        assert_eq!(value.as_number(), Some(42.0));
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
}
