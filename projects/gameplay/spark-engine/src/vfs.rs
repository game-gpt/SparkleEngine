//! 模组资源虚拟路径：限制在模组根目录内。

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use spark_core::{ErrorArg, SparkError, codes};

use crate::EngineError;

#[derive(Debug, Clone)]
pub struct ModVfs {
    pub mod_id: String,
    pub root: PathBuf,
}

impl ModVfs {
    pub fn new(mod_id: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            mod_id: mod_id.into(),
            root: root.into(),
        }
    }

    /// 解析相对路径；禁止 `..` 逃逸出模组根。
    pub fn resolve(&self, rel: &str) -> Result<PathBuf, SparkError> {
        let rel = rel.trim_start_matches(['/', '\\']);
        let mut out = self.root.clone();
        for c in Path::new(rel).components() {
            match c {
                Component::Normal(s) => out.push(s),
                Component::CurDir => {}
                Component::ParentDir => {
                    return Err(SparkError::new(codes::vfs_path_invalid())
                        .arg("mod_id", ErrorArg::String(Arc::from(self.mod_id.as_str())))
                        .arg("path", ErrorArg::Path(Arc::from(rel)))
                        .arg("reason", ErrorArg::String(Arc::from("parent_dir"))));
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(SparkError::new(codes::vfs_path_invalid())
                        .arg("mod_id", ErrorArg::String(Arc::from(self.mod_id.as_str())))
                        .arg("path", ErrorArg::Path(Arc::from(rel)))
                        .arg("reason", ErrorArg::String(Arc::from("absolute"))));
                }
            }
        }
        let root = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
        match out.canonicalize() {
            Ok(canon) if canon.starts_with(&root) => Ok(canon),
            Ok(_) => Err(SparkError::new(codes::vfs_path_invalid())
                .arg("mod_id", ErrorArg::String(Arc::from(self.mod_id.as_str())))
                .arg("path", ErrorArg::Path(Arc::from(rel)))
                .arg("reason", ErrorArg::String(Arc::from("escape")))),
            // 文件尚不存在时仍返回规范化拼接路径（不 canonicalize）
            Err(_) => {
                if out.starts_with(&self.root) {
                    Ok(out)
                } else {
                    Err(SparkError::new(codes::vfs_path_invalid())
                        .arg("mod_id", ErrorArg::String(Arc::from(self.mod_id.as_str())))
                        .arg("path", ErrorArg::Path(Arc::from(rel)))
                        .arg("reason", ErrorArg::String(Arc::from("escape"))))
                }
            }
        }
    }

    pub fn read_to_string(&self, rel: &str) -> Result<String, EngineError> {
        let p = self.resolve(rel)?;
        std::fs::read_to_string(&p)
            .map_err(|e| EngineError::io(p.display().to_string(), e.to_string()))
    }

    pub fn read_bytes(&self, rel: &str) -> Result<Vec<u8>, EngineError> {
        let p = self.resolve(rel)?;
        std::fs::read(&p).map_err(|e| EngineError::io(p.display().to_string(), e.to_string()))
    }
}
