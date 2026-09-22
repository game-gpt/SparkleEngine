//! EditSession dry-run / apply。

use std::fs;

use spark_asset::AssetMetaStore;
use spark_edit::{EditMode, EditOp, EditPlan, EditSession, TransactionState};
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
