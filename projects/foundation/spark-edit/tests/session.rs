//! EditSession dry-run / apply。

use std::fs;

use spark_asset::AssetMetaStore;
use spark_edit::{
    EditCapabilities, EditMode, EditOp, EditPlan, EditSession, TransactionState, host_names, parse_edit_source,
};
use spark_prefab::PrefabDocument;
use uuid::Uuid;

#[test]
fn parse_von_edit_plan() {
    let plan = EditPlan {
        name: Some("demo".into()),
        ops: vec![
            EditOp::MetaCreate {
                path: "assets/x.png".into(),
            },
            EditOp::PrefabEnsure {
                path: "assets/player.von".into(),
                root: "player".into(),
            },
        ],
    };
    let text = oak_von::to_string(&plan).expect("ser");
    let back = EditPlan::from_von(&text).expect(&format!("parse from: {text}"));
    assert_eq!(back.name.as_deref(), Some("demo"));
    assert_eq!(back.ops.len(), 2);
    assert!(matches!(back.ops[0], EditOp::MetaCreate { .. }));
}

#[test]
fn dry_run_prefab_plan_does_not_write() {
    let dir = std::env::temp_dir().join(format!("spark-edit-dry-{}", Uuid::now_v7()));
    fs::create_dir_all(dir.join("assets")).unwrap();

    let plan = EditPlan {
        name: Some("player".into()),
        ops: vec![
            EditOp::PrefabEnsure {
                path: "assets/player.von".into(),
                root: "player".into(),
            },
            EditOp::PrefabEnsureNode {
                path: "assets/player.von".into(),
                id: "sprite".into(),
                parent: Some("player".into()),
            },
            EditOp::PrefabEnsureComponent {
                path: "assets/player.von".into(),
                node: "sprite".into(),
                component: "Sprite".into(),
            },
            EditOp::PrefabSave {
                path: "assets/player.von".into(),
            },
        ],
    };

    let mut session = EditSession::new(&dir, EditMode::DryRun);
    let report = session.run(&plan);
    assert!(report.ok, "{:?}", report.diagnostics);
    assert_eq!(report.transaction, TransactionState::Planned);
    assert!(!dir.join("assets/player.von").exists());
    assert!(!report.changes.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn apply_writes_prefab_and_meta() {
    let dir = std::env::temp_dir().join(format!("spark-edit-apply-{}", Uuid::now_v7()));
    fs::create_dir_all(dir.join("assets")).unwrap();

    let plan = EditPlan {
        name: None,
        ops: vec![
            EditOp::PrefabEnsure {
                path: "assets/player.von".into(),
                root: "player".into(),
            },
            EditOp::PrefabSave {
                path: "assets/player.von".into(),
            },
        ],
    };

    let mut session = EditSession::new(&dir, EditMode::Apply);
    let report = session.run(&plan);
    assert!(report.ok, "{:?}", report.diagnostics);
    assert_eq!(report.transaction, TransactionState::Applied);

    let prefab = dir.join("assets/player.von");
    assert!(prefab.is_file());
    assert!(AssetMetaStore::path(&prefab).is_file());
    let doc = PrefabDocument::load(&prefab).unwrap();
    assert_eq!(doc.root, "player");
    assert_eq!(AssetMetaStore::load(&prefab).unwrap().kind.as_deref(), Some("prefab"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn apply_rename_keeps_guid() {
    let dir = std::env::temp_dir().join(format!("spark-edit-rename-{}", Uuid::now_v7()));
    fs::create_dir_all(dir.join("assets")).unwrap();
    let src = dir.join("assets/a.png");
    fs::write(&src, b"png").unwrap();
    let meta = AssetMetaStore::create(&src).unwrap();

    let plan = EditPlan {
        name: None,
        ops: vec![EditOp::AssetRename {
            from: "assets/a.png".into(),
            to: "assets/b.png".into(),
        }],
    };
    let mut session = EditSession::new(&dir, EditMode::Apply);
    let report = session.run(&plan);
    assert!(report.ok, "{:?}", report.diagnostics);
    assert!(!src.exists());
    let dst = dir.join("assets/b.png");
    assert!(dst.is_file());
    assert_eq!(AssetMetaStore::load(&dst).unwrap().guid, meta.guid);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn index_scan_reports_count() {
    let dir = std::env::temp_dir().join(format!("spark-edit-scan-{}", Uuid::now_v7()));
    fs::create_dir_all(dir.join("assets")).unwrap();
    let asset = dir.join("assets/c.png");
    fs::write(&asset, b"x").unwrap();
    AssetMetaStore::create(&asset).unwrap();

    let plan = EditPlan {
        name: None,
        ops: vec![EditOp::IndexScan],
    };
    let mut session = EditSession::new(&dir, EditMode::Check);
    let report = session.run(&plan);
    assert!(report.ok, "{:?}", report.diagnostics);
    assert!(report.diagnostics.iter().any(|d| d.code == "spark.edit.index_scanned"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn apply_denied_without_write_capability() {
    let dir = std::env::temp_dir().join(format!("spark-edit-cap-{}", Uuid::now_v7()));
    fs::create_dir_all(&dir).unwrap();
    let plan = EditPlan {
        name: None,
        ops: vec![EditOp::IndexScan],
    };
    let mut session = EditSession::new(&dir, EditMode::Apply).with_capabilities(EditCapabilities::read_only());
    let report = session.run(&plan);
    assert!(!report.ok);
    assert!(report.diagnostics.iter().any(|d| d.code == "spark.edit.capability_denied"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn host_calls_lower_to_ops() {
    use std::collections::BTreeMap;
    use spark_asset::MetaValue;
    use spark_edit::EditHostScript;
    use spark_edit::HostCall;

    let script = EditHostScript {
        profile: Some("spark-edit-1".into()),
        transaction: Some("demo".into()),
        calls: vec![
            HostCall {
                name: host_names::PREFAB_ENSURE.into(),
                args: BTreeMap::from([
                    ("path".into(), MetaValue::String("assets/player.von".into())),
                    ("root".into(), MetaValue::String("player".into())),
                ]),
            },
            HostCall {
                name: host_names::PREFAB_SAVE.into(),
                args: BTreeMap::from([("path".into(), MetaValue::String("assets/player.von".into()))]),
            },
        ],
    };
    let text = oak_von::to_string(&script).expect("ser");
    let plan = parse_edit_source(&text).expect(&format!("parse host script from {text}"));
    assert_eq!(plan.name.as_deref(), Some("demo"));
    assert_eq!(plan.ops.len(), 2);
    assert!(matches!(plan.ops[0], EditOp::PrefabEnsure { .. }));
    assert!(matches!(plan.ops[1], EditOp::PrefabSave { .. }));
}
