//! 从当前目录的 `package.json` 识别 Spark 游戏项目。
//!
//! 权威字段在 `spark.*`；未声明 `kind` 时按 `Cargo.toml` 与
//! `assets/scripts/*.script` 推断，两者并存或皆无则报错，要求显式 hybrid / kind。

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// 项目脚本/原生组合形态（决定 Hierarchy 演示树与 Play 兜底样例）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    /// 仅原生 Rust（`Cargo.toml`，无 `*.script`）。
    Rust,
    /// 仅 Valkyrie / Sparkle Script（`assets/scripts/*.script`）。
    Valkyrie,
    /// Rust 与 Script 并存（须在 `package.json` 显式声明，不可静默推断）。
    Hybrid,
}

impl ProjectKind {
    /// 机器可读标识：`rust` / `valkyrie` / `hybrid`。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Valkyrie => "valkyrie",
            Self::Hybrid => "hybrid",
        }
    }

    /// 菜单/状态栏用中文标签。
    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "纯 Rust",
            Self::Valkyrie => "纯 Valkyrie",
            Self::Hybrid => "Rust + Valkyrie",
        }
    }
}

/// 已解析的项目根与 `spark` 配置快照。
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    /// 项目根目录（含 `package.json`）。
    pub root: PathBuf,
    /// `package.json` 的 `name`，缺省时用目录名。
    pub name: String,
    /// 解析或推断得到的 [`ProjectKind`]。
    pub kind: ProjectKind,
    /// `true` 表示 `kind` 来自磁盘推断而非 `spark.kind`。
    pub kind_inferred: bool,
    /// `spark.startupScene`（相对资源路径，可缺）。
    pub startup_scene: Option<String>,
    /// `spark.scriptEntry`（脚本入口提示，可缺）。
    pub script_entry: Option<String>,
    /// `spark.cargoManifest`（Cargo 清单路径提示，可缺）。
    pub cargo_manifest: Option<String>,
    /// `spark.runTarget`：Play 时优先匹配的样例名（如 `snake`）。
    pub run_target: Option<String>,
    /// `dependencies` / `devDependencies` 是否含 `@game-gpt/sparkle-engine`。
    pub has_sparkle_engine_dep: bool,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    dependencies: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<serde_json::Map<String, serde_json::Value>>,
    spark: Option<SparkField>,
}

#[derive(Debug, Deserialize)]
struct SparkField {
    kind: Option<String>,
    #[serde(rename = "startupScene")]
    startup_scene: Option<String>,
    #[serde(rename = "scriptEntry")]
    script_entry: Option<String>,
    #[serde(rename = "cargoManifest")]
    cargo_manifest: Option<String>,
    #[serde(rename = "runTarget")]
    run_target: Option<String>,
}

/// 打开/识别项目失败原因（面向 CLI 打印）。
#[derive(Debug)]
pub enum ProjectError {
    /// 起点目录下没有 `package.json`。
    NoPackageJson {
        /// 实际搜索的项目根。
        searched: PathBuf,
    },
    /// `package.json` 读盘或 JSON 解析失败。
    InvalidJson {
        /// 出错文件路径。
        path: PathBuf,
        /// 底层错误摘要。
        detail: String,
    },
    /// 无法唯一推断 `spark.kind`（需用户显式声明）。
    AmbiguousKind {
        /// 说明与建议文案。
        detail: String,
    },
    /// `spark.kind` 取值不在允许集合。
    InvalidKind {
        /// 用户给出的原始字符串。
        value: String,
    },
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPackageJson { searched } => write!(
                f,
                "当前目录不是 Spark 游戏项目。\n未找到 package.json（搜索起点：{}）。\n请在已安装 @game-gpt/sparkle-engine 的项目目录中运行 spark studio。",
                searched.display()
            ),
            Self::InvalidJson { path, detail } => {
                write!(f, "无法解析 {}：{detail}", path.display())
            }
            Self::AmbiguousKind { detail } => write!(f, "{detail}\n请在 package.json 中设置：\n\"spark\": {{ \"kind\": \"hybrid\" }}"),
            Self::InvalidKind { value } => write!(f, "未知 spark.kind = \"{value}\"。允许：rust | valkyrie | hybrid"),
        }
    }
}

/// 解析 `--cwd` / 位置参数 / 当前目录为项目根。
pub fn resolve_project_dir(args: &[String]) -> PathBuf {
    let mut cwd: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--cwd" {
            if let Some(p) = args.get(i + 1) {
                cwd = Some(PathBuf::from(p));
                i += 2;
                continue;
            }
        }
        else if let Some(rest) = a.strip_prefix("--cwd=") {
            cwd = Some(PathBuf::from(rest));
        }
        else if a == "--safe-mode" || a == "--help" || a == "-h" || a == "--play" {
            // 忽略（由 main 处理 --play）
        }
        else if !a.starts_with('-') && cwd.is_none() {
            cwd = Some(PathBuf::from(a));
        }
        i += 1;
    }
    cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn has_cargo(root: &Path) -> bool {
    root.join("Cargo.toml").is_file()
}

fn has_script_files(root: &Path) -> bool {
    let scripts = root.join("assets").join("scripts");
    let Ok(rd) = std::fs::read_dir(scripts)
    else {
        return false;
    };
    rd.filter_map(|e| e.ok()).any(|e| e.path().extension().is_some_and(|ext| ext == "script"))
}

fn parse_kind(raw: &str) -> Result<ProjectKind, ProjectError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "rust" => Ok(ProjectKind::Rust),
        "valkyrie" | "script" => Ok(ProjectKind::Valkyrie),
        "hybrid" => Ok(ProjectKind::Hybrid),
        other => Err(ProjectError::InvalidKind { value: other.to_string() }),
    }
}

fn infer_kind(root: &Path) -> Result<(ProjectKind, bool), ProjectError> {
    let cargo = has_cargo(root);
    let scripts = has_script_files(root);
    match (cargo, scripts) {
        (true, false) => Ok((ProjectKind::Rust, true)),
        (false, true) => Ok((ProjectKind::Valkyrie, true)),
        (true, true) => Err(ProjectError::AmbiguousKind {
            detail: "检测到 Cargo.toml 与 assets/scripts/*.script，但 package.json 未声明 spark.kind。".into(),
        }),
        (false, false) => Err(ProjectError::AmbiguousKind {
            detail: "未检测到 Cargo.toml 或 assets/scripts/*.script，且未声明 spark.kind。".into(),
        }),
    }
}

/// 读取 `root/package.json`，解析 `spark` 字段并填充 [`ProjectInfo`]。
///
/// 缺少 `spark.kind` 时调用磁盘推断；推断失败返回 [`ProjectError::AmbiguousKind`]。
pub fn load_project(root: &Path) -> Result<ProjectInfo, ProjectError> {
    let pkg_path = root.join("package.json");
    if !pkg_path.is_file() {
        return Err(ProjectError::NoPackageJson { searched: root.to_path_buf() });
    }
    let text = std::fs::read_to_string(&pkg_path).map_err(|e| ProjectError::InvalidJson { path: pkg_path.clone(), detail: e.to_string() })?;
    let pkg: PackageJson = serde_json::from_str(&text).map_err(|e| ProjectError::InvalidJson { path: pkg_path, detail: e.to_string() })?;

    let has_dep =
        |map: &Option<serde_json::Map<String, serde_json::Value>>| map.as_ref().is_some_and(|m| m.contains_key("@game-gpt/sparkle-engine"));
    let has_sparkle_engine_dep = has_dep(&pkg.dependencies) || has_dep(&pkg.dev_dependencies);

    let (kind, kind_inferred) =
        if let Some(k) = pkg.spark.as_ref().and_then(|s| s.kind.as_deref()) { (parse_kind(k)?, false) } else { infer_kind(root)? };

    Ok(ProjectInfo {
        root: root.to_path_buf(),
        name: pkg.name.unwrap_or_else(|| root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "game".into())),
        kind,
        kind_inferred,
        startup_scene: pkg.spark.as_ref().and_then(|s| s.startup_scene.clone()),
        script_entry: pkg.spark.as_ref().and_then(|s| s.script_entry.clone()),
        cargo_manifest: pkg.spark.as_ref().and_then(|s| s.cargo_manifest.clone()),
        run_target: pkg.spark.as_ref().and_then(|s| s.run_target.clone()),
        has_sparkle_engine_dep,
    })
}

/// 列出 `assets/` 下一层目录与文件名（演示用浅扫描）。
pub fn list_asset_entries(root: &Path) -> Vec<String> {
    let assets = root.join("assets");
    let mut out = Vec::new();
    if !assets.is_dir() {
        return out;
    }
    out.push("assets".into());
    if let Ok(rd) = std::fs::read_dir(&assets) {
        let mut names: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        for n in names {
            out.push(format!("  {n}/"));
            let sub = assets.join(&n);
            if sub.is_dir() {
                if let Ok(files) = std::fs::read_dir(&sub) {
                    let mut files: Vec<_> = files
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_file())
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .collect();
                    files.sort();
                    for f in files.into_iter().take(12) {
                        out.push(format!("    {f}"));
                    }
                }
            }
        }
    }
    out
}
