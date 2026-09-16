//! Oaks `oak-valkyrie` 解析 → `spark-vm` 字节码。
//!
//! 语法以 Valkyrie 为准（`micro` / `let` / 表达式）。游戏绑定经原生函数表注入。

mod compile;

use oak_core::{Builder, SourceText};
use oak_valkyrie::{ValkyrieBuilder, ValkyrieLanguage, ValkyrieRoot, ast::StatementNode};
use spark_vm::Module;
use thiserror::Error;

pub use compile::compile_root;

#[derive(Debug, Error)]
pub enum ValkyrieScriptError {
    #[error("解析错误：{0}")]
    Parse(String),
    #[error("编译错误：{0}")]
    Compile(String),
}

/// 源码 → [`Module`]。
pub fn compile(source: &str, natives: &[&str]) -> Result<Module, ValkyrieScriptError> {
    let root = parse(source)?;
    compile_root(&root, natives).map_err(ValkyrieScriptError::Compile)
}

/// 解析为 AST 根。
pub fn parse(source: &str) -> Result<ValkyrieRoot, ValkyrieScriptError> {
    // 允许 legacy `fn` 别名；ECS/Widget 扩展留给后续绑定层开启。
    let language = ValkyrieLanguage {
        allow_legacy_function: true,
        ..ValkyrieLanguage::default()
    };
    let builder = ValkyrieBuilder::new(&language);
    let text = SourceText::new(source);
    let mut session = oak_core::ParseSession::<ValkyrieLanguage>::default();
    let out = builder.build(&text, &[], &mut session);
    match out.result {
        Ok(root) => Ok(root),
        Err(e) => {
            let mut msg = e.to_string();
            if !out.diagnostics.is_empty() {
                let soft: Vec<String> = out.diagnostics.iter().map(|d| format!("{d:?}")).collect();
                msg.push_str("; ");
                msg.push_str(&soft.join("; "));
            }
            Err(ValkyrieScriptError::Parse(msg))
        }
    }
}

/// 调试：列出根上 `micro` 名。
pub fn list_micros(source: &str) -> Result<Vec<String>, ValkyrieScriptError> {
    let root = parse(source)?;
    Ok(root
        .items
        .iter()
        .filter_map(|i| match i {
            StatementNode::Micro(m) => Some(m.name.name.clone()),
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_vm::{StdHost, Vm};

    #[test]
    fn arithmetic_main() {
        let m = compile("return 40 + 2", &[]).unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn micro_call() {
        let m = compile(
            r#"
            micro add(a, b) {
                return a + b
            }
            return add(40, 2)
            "#,
            &[],
        )
        .unwrap();
        let mut vm = Vm::new(m);
        let v = vm.run(&mut StdHost).unwrap();
        assert_eq!(v.as_number(), Some(42.0));
    }
}
