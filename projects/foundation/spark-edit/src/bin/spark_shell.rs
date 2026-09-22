//! `spark-shell`：Edit Runtime CLI 入口（由 `spark shell` 转发）。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use spark_edit::{EditMode, EditPlan, EditSession};

fn usage() -> ExitCode {
    eprintln!(
        "Usage:
  spark-shell --path <plan.edit.von> [--mode check|dry-run|apply] [--cwd <dir>] [--json]
  spark-shell --code <von-text> [--mode ...] [--cwd <dir>] [--json]

Edit plans are VON documents with an `ops` list. Sparkle Script will target the same runtime."
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        return usage();
    }

    let mut cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut mode = EditMode::DryRun;
    let mut json = false;
    let mut path: Option<PathBuf> = None;
    let mut code: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--cwd" => {
                if i + 1 >= args.len() {
                    eprintln!("--cwd requires a path");
                    return ExitCode::from(2);
                }
                cwd = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            a if a.starts_with("--cwd=") => {
                cwd = PathBuf::from(&a["--cwd=".len()..]);
                i += 1;
            }
            "--mode" => {
                if i + 1 >= args.len() {
                    eprintln!("--mode requires check|dry-run|apply");
                    return ExitCode::from(2);
                }
                mode = match EditMode::parse(&args[i + 1]) {
                    Some(m) => m,
                    None => {
                        eprintln!("unknown mode: {}", args[i + 1]);
                        return ExitCode::from(2);
                    }
                };
                i += 2;
            }
            a if a.starts_with("--mode=") => {
                mode = match EditMode::parse(&a["--mode=".len()..]) {
                    Some(m) => m,
                    None => {
                        eprintln!("unknown mode");
                        return ExitCode::from(2);
                    }
                };
                i += 1;
            }
            "--path" => {
                if i + 1 >= args.len() {
                    eprintln!("--path requires a file");
                    return ExitCode::from(2);
                }
                path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            a if a.starts_with("--path=") => {
                path = Some(PathBuf::from(&a["--path=".len()..]));
                i += 1;
            }
            "--code" => {
                if i + 1 >= args.len() {
                    eprintln!("--code requires VON text");
                    return ExitCode::from(2);
                }
                code = Some(args[i + 1].clone());
                i += 2;
            }
            a if a.starts_with("--code=") => {
                code = Some(a["--code=".len()..].to_string());
                i += 1;
            }
            "--json" => {
                json = true;
                i += 1;
            }
            "--check" => {
                mode = EditMode::Check;
                i += 1;
            }
            "--dry-run" => {
                mode = EditMode::DryRun;
                i += 1;
            }
            "--apply" => {
                mode = EditMode::Apply;
                i += 1;
            }
            other => {
                eprintln!("unknown argument: {other}");
                return usage();
            }
        }
    }

    let text = if let Some(code) = code {
        code
    } else if let Some(path) = path {
        let abs = if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        };
        match fs::read_to_string(&abs) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("failed to read {}: {e}", abs.display());
                return ExitCode::from(1);
            }
        }
    } else {
        eprintln!("provide --path or --code");
        return usage();
    };

    let plan = match EditPlan::from_von(&text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("spark.edit.plan_parse: {e}");
            return ExitCode::from(1);
        }
    };

    let mut session = EditSession::new(cwd, mode);
    let report = session.run(&plan);
    if json {
        println!("{}", report.to_json());
    } else {
        println!("ok={} mode={} transaction={:?}", report.ok, report.mode, report.transaction);
        for c in &report.changes {
            println!("  change {:?} {}", c.kind, c.path);
        }
        for d in &report.diagnostics {
            println!("  {:?} {} {}", d.severity, d.code, d.message);
        }
    }
    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
