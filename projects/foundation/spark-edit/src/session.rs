//! 编辑会话：按模式执行 [`EditPlan`]。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use spark_asset::{AssetIndex, AssetMetaStore, MetaValue};
use spark_prefab::{PrefabDocument, save_registered, validate_prefab_file};

use crate::capabilities::EditCapabilities;
use crate::diagnostic::{Diagnostic, Severity};
use crate::op::EditOp;
use crate::plan::EditPlan;
use crate::report::{ChangeKind, ChangeRecord, EditReport, TransactionState};

/// 执行模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    /// 解析与校验，不写盘、不生成变更计划以外的副作用。
    Check,
    /// 内存执行并收集变更计划，不写盘。
    DryRun,
    /// 真正写盘。
    Apply,
}

impl EditMode {
    /// CLI / JSON 用小写名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::DryRun => "dry-run",
            Self::Apply => "apply",
        }
    }

    /// 解析模式字符串。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "check" => Some(Self::Check),
            "dry-run" | "dryrun" | "dry_run" => Some(Self::DryRun),
            "apply" => Some(Self::Apply),
            _ => None,
        }
    }
}

/// 项目编辑会话。
pub struct EditSession {
    root: PathBuf,
    mode: EditMode,
    caps: EditCapabilities,
    prefabs: BTreeMap<String, PrefabDocument>,
    diagnostics: Vec<Diagnostic>,
    changes: Vec<ChangeRecord>,
}

impl EditSession {
    /// 在项目根上打开会话（默认 [`EditCapabilities::project_edit`]）。
    pub fn new(root: impl Into<PathBuf>, mode: EditMode) -> Self {
        Self {
            root: root.into(),
            mode,
            caps: EditCapabilities::project_edit(),
            prefabs: BTreeMap::new(),
            diagnostics: Vec::new(),
            changes: Vec::new(),
        }
    }

    /// 覆盖能力集。
    pub fn with_capabilities(mut self, caps: EditCapabilities) -> Self {
        self.caps = caps;
        self
    }

    /// 执行计划并返回报告。
    pub fn run(&mut self, plan: &EditPlan) -> EditReport {
        if self.mode == EditMode::Apply && !self.caps.allows_apply() {
            self.push_err(
                "spark.edit.capability_denied",
                "apply requires write-assets / project-edit capability",
                None,
            );
            return EditReport {
                ok: false,
                mode: self.mode.as_str().into(),
                diagnostics: self.diagnostics.clone(),
                changes: Vec::new(),
                transaction: TransactionState::RolledBack,
            };
        }
        for op in &plan.ops {
            if self.has_error() {
                break;
            }
            self.exec_op(op);
        }
        let ok = !self.has_error();
        let transaction = if !ok {
            TransactionState::RolledBack
        } else {
            match self.mode {
                EditMode::Check => TransactionState::Checked,
                EditMode::DryRun => TransactionState::Planned,
                EditMode::Apply => TransactionState::Applied,
            }
        };
        EditReport {
            ok,
            mode: self.mode.as_str().into(),
            diagnostics: self.diagnostics.clone(),
            changes: self.changes.clone(),
            transaction,
        }
    }

    fn has_error(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.root.join(p)
        }
    }

    fn rel_display(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
    }

    fn push_err(&mut self, code: &str, message: impl Into<String>, path: Option<String>) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            path,
        });
    }

    fn push_info(&mut self, code: &str, message: impl Into<String>, path: Option<String>) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Info,
            code: code.into(),
            message: message.into(),
            path,
        });
    }

    fn note_change(&mut self, kind: ChangeKind, path: &Path) {
        self.changes.push(ChangeRecord {
            kind,
            path: self.rel_display(path),
        });
    }

    fn exec_op(&mut self, op: &EditOp) {
        match op {
            EditOp::MetaCreate { path } => self.op_meta_create(path),
            EditOp::MetaLoad { path } => self.op_meta_load(path),
            EditOp::AssetRename { from, to } => self.op_asset_rename(from, to),
            EditOp::IndexScan => self.op_index_scan(),
            EditOp::PrefabEnsure { path, root } => self.op_prefab_ensure(path, root),
            EditOp::PrefabEnsureNode { path, id, parent } => self.op_prefab_ensure_node(path, id, parent.as_deref()),
            EditOp::PrefabEnsureComponent {
                path,
                node,
                component,
            } => self.op_prefab_ensure_component(path, node, component),
            EditOp::PrefabSetField {
                path,
                node,
                component,
                field,
                value,
            } => self.op_prefab_set_field(path, node, component, field, value),
            EditOp::PrefabValidate { path } => self.op_prefab_validate(path),
            EditOp::PrefabSave { path } => self.op_prefab_save(path),
        }
    }

    fn op_meta_create(&mut self, path: &str) {
        let abs = self.resolve(path);
        let meta_path = AssetMetaStore::path(&abs);
        match self.mode {
            EditMode::Check => {
                if meta_path.is_file() {
                    self.push_err(
                        "spark.asset.meta_already_exists",
                        "sidecar already exists",
                        Some(self.rel_display(&meta_path)),
                    );
                } else {
                    self.push_info("spark.edit.meta_create_ok", "meta.create would succeed", Some(path.into()));
                }
            }
            EditMode::DryRun => {
                if meta_path.is_file() {
                    self.push_err(
                        "spark.asset.meta_already_exists",
                        "sidecar already exists",
                        Some(self.rel_display(&meta_path)),
                    );
                } else {
                    self.note_change(ChangeKind::WriteMeta, &meta_path);
                }
            }
            EditMode::Apply => match AssetMetaStore::create(&abs) {
                Ok(_) => self.note_change(ChangeKind::WriteMeta, &meta_path),
                Err(e) => self.push_err(e.code(), e.to_string(), Some(path.into())),
            },
        }
    }

    fn op_meta_load(&mut self, path: &str) {
        let abs = self.resolve(path);
        match AssetMetaStore::load(&abs) {
            Ok(meta) => self.push_info(
                "spark.edit.meta_loaded",
                format!("guid={}", meta.guid),
                Some(path.into()),
            ),
            Err(e) => self.push_err(e.code(), e.to_string(), Some(path.into())),
        }
    }

    fn op_asset_rename(&mut self, from: &str, to: &str) {
        let from_abs = self.resolve(from);
        let to_abs = self.resolve(to);
        match self.mode {
            EditMode::Check => {
                if !from_abs.is_file() && !AssetMetaStore::path(&from_abs).is_file() {
                    self.push_err("spark.asset.meta_missing", "rename source missing", Some(from.into()));
                } else if AssetMetaStore::path(&to_abs).is_file() {
                    self.push_err(
                        "spark.asset.meta_already_exists",
                        "rename target meta exists",
                        Some(to.into()),
                    );
                } else {
                    self.push_info("spark.edit.rename_ok", "asset.rename would succeed", Some(from.into()));
                }
            }
            EditMode::DryRun => {
                if !from_abs.is_file() && !AssetMetaStore::path(&from_abs).is_file() {
                    self.push_err("spark.asset.meta_missing", "rename source missing", Some(from.into()));
                } else if AssetMetaStore::path(&to_abs).is_file() {
                    self.push_err(
                        "spark.asset.meta_already_exists",
                        "rename target meta exists",
                        Some(to.into()),
                    );
                } else {
                    self.note_change(ChangeKind::Update, &from_abs);
                    self.note_change(ChangeKind::Create, &to_abs);
                    self.note_change(ChangeKind::WriteMeta, &AssetMetaStore::path(&to_abs));
                }
            }
            EditMode::Apply => {
                let mut index = match AssetIndex::scan(&self.root) {
                    Ok(idx) => idx,
                    Err(e) => {
                        self.push_err(e.code(), e.to_string(), None);
                        return;
                    }
                };
                if index.guid_of(&from_abs).is_none() {
                    if let Err(e) = index.register(&from_abs) {
                        self.push_err(e.code(), e.to_string(), Some(from.into()));
                        return;
                    }
                }
                match index.rename(&from_abs, &to_abs) {
                    Ok(()) => {
                        self.note_change(ChangeKind::Update, &to_abs);
                        self.note_change(ChangeKind::WriteMeta, &AssetMetaStore::path(&to_abs));
                    }
                    Err(e) => self.push_err(e.code(), e.to_string(), Some(from.into())),
                }
            }
        }
    }

    fn op_index_scan(&mut self) {
        match AssetIndex::scan(&self.root) {
            Ok(index) => self.push_info(
                "spark.edit.index_scanned",
                format!("entries={}", index.len()),
                Some(self.rel_display(&self.root)),
            ),
            Err(e) => self.push_err(e.code(), e.to_string(), None),
        }
    }

    fn op_prefab_validate(&mut self, path: &str) {
        let abs = self.resolve(path);
        match validate_prefab_file(&abs) {
            Ok(_) => self.push_info("spark.edit.prefab_valid", "prefab.validate ok", Some(path.into())),
            Err(e) => self.push_err(e.code(), e.to_string(), Some(path.into())),
        }
    }

    fn load_or_open_prefab(&mut self, path: &str, root: Option<&str>) -> Option<()> {
        if self.prefabs.contains_key(path) {
            return Some(());
        }
        let abs = self.resolve(path);
        if abs.is_file() {
            match PrefabDocument::load(&abs) {
                Ok(doc) => {
                    self.prefabs.insert(path.into(), doc);
                    Some(())
                }
                Err(e) => {
                    self.push_err(e.code(), e.to_string(), Some(path.into()));
                    None
                }
            }
        } else if let Some(root) = root {
            self.prefabs.insert(path.into(), PrefabDocument::new(root));
            Some(())
        } else {
            self.push_err(
                "spark.prefab.io",
                "prefab file missing; use prefab.ensure first",
                Some(path.into()),
            );
            None
        }
    }

    fn op_prefab_ensure(&mut self, path: &str, root: &str) {
        if self.prefabs.contains_key(path) {
            return;
        }
        let abs = self.resolve(path);
        if abs.is_file() {
            let _ = self.load_or_open_prefab(path, None);
        } else {
            self.prefabs.insert(path.into(), PrefabDocument::new(root));
            if self.mode != EditMode::Check {
                self.note_change(ChangeKind::Create, &abs);
            }
        }
    }

    fn op_prefab_ensure_node(&mut self, path: &str, id: &str, parent: Option<&str>) {
        if self.load_or_open_prefab(path, None).is_none() {
            return;
        }
        let Some(doc) = self.prefabs.get_mut(path) else {
            return;
        };
        doc.ensure_node(id);
        if let Some(parent) = parent {
            if let Err(e) = doc.ensure_child(parent, id) {
                self.push_err(e.code(), e.to_string(), Some(path.into()));
            }
        }
        if self.mode != EditMode::Check {
            self.note_change(ChangeKind::Update, &self.resolve(path));
        }
    }

    fn op_prefab_ensure_component(&mut self, path: &str, node: &str, component: &str) {
        if self.load_or_open_prefab(path, None).is_none() {
            return;
        }
        let Some(doc) = self.prefabs.get_mut(path) else {
            return;
        };
        if let Err(e) = doc.ensure_component(node, component) {
            self.push_err(e.code(), e.to_string(), Some(path.into()));
            return;
        }
        if self.mode != EditMode::Check {
            self.note_change(ChangeKind::Update, &self.resolve(path));
        }
    }

    fn op_prefab_set_field(
        &mut self,
        path: &str,
        node: &str,
        component: &str,
        field: &str,
        value: &MetaValue,
    ) {
        if self.load_or_open_prefab(path, None).is_none() {
            return;
        }
        let Some(doc) = self.prefabs.get_mut(path) else {
            return;
        };
        if let Err(e) = doc.ensure_component(node, component) {
            self.push_err(e.code(), e.to_string(), Some(path.into()));
            return;
        }
        let Some(comp) = doc.nodes.get_mut(node).and_then(|n| n.components.get_mut(component)) else {
            self.push_err("spark.prefab.node_missing", "component missing after ensure", Some(path.into()));
            return;
        };
        match comp {
            MetaValue::Table(map) => {
                map.insert(field.into(), value.clone());
            }
            other => {
                let mut map = BTreeMap::new();
                map.insert(field.into(), value.clone());
                *other = MetaValue::Table(map);
            }
        }
        if self.mode != EditMode::Check {
            self.note_change(ChangeKind::Update, &self.resolve(path));
        }
    }

    fn op_prefab_save(&mut self, path: &str) {
        if self.load_or_open_prefab(path, None).is_none() {
            return;
        }
        let Some(doc) = self.prefabs.get(path).cloned() else {
            return;
        };
        if let Err(e) = doc.validate() {
            self.push_err(e.code(), e.to_string(), Some(path.into()));
            return;
        }
        let abs = self.resolve(path);
        let meta_path = AssetMetaStore::path(&abs);
        match self.mode {
            EditMode::Check => {
                self.push_info("spark.edit.prefab_save_ok", "prefab.save would succeed", Some(path.into()));
            }
            EditMode::DryRun => {
                let kind = if abs.is_file() {
                    ChangeKind::Update
                } else {
                    ChangeKind::Create
                };
                self.note_change(kind, &abs);
                self.note_change(ChangeKind::WriteMeta, &meta_path);
            }
            EditMode::Apply => match save_registered(&doc, &abs) {
                Ok(_) => {
                    let kind = ChangeKind::Update;
                    self.note_change(kind, &abs);
                    self.note_change(ChangeKind::WriteMeta, &meta_path);
                }
                Err(e) => self.push_err(e.code(), e.to_string(), Some(path.into())),
            },
        }
    }
}
