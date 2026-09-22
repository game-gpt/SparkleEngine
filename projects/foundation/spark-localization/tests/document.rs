//! 自 `src/document.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::*;

#[test]
fn template_sugar_splits_arguments() {
    let def = MessageDefinition::from_template_sugar("欢迎回来，{player_name}");
    match def {
        MessageDefinition::Pattern(nodes) => {
            assert_eq!(nodes.len(), 2);
            assert!(matches!(&nodes[0], MessageNode::Text(t) if t.as_ref() == "欢迎回来，"));
            assert!(matches!(
                &nodes[1],
                MessageNode::Argument { name, .. } if name.as_ref() == "player_name"
            ));
        }
        other => panic!("unexpected {other:?}"),
    }
}
