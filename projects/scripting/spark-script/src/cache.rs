//! 编译制品内存缓存（按源码 / profile / 宿主 schema 指纹）。

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use crate::{artifact::ARTIFACT_FORMAT_VERSION, compiler::CompiledPackage, request::CompilationRequest};

/// 进程内编译缓存。键含源码哈希、语言 profile、宿主 schema、制品格式版本。
#[derive(Debug, Default)]
pub struct ArtifactCache {
    entries: HashMap<u64, CompiledPackage>,
    /// 命中次数（`get` 找到条目时递增）。
    pub hits: u64,
    /// 未命中次数（`get` 未找到时递增）。
    pub misses: u64,
}

impl ArtifactCache {
    /// 空缓存；命中计数归零。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前缓存条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否无任何条目。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 清空条目并重置命中 / 未命中计数。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// 缓存键：源码 + 语言契约 + 宿主 ABI + 优化 / 确定性 + 制品格式。
    pub fn key_for(request: &CompilationRequest, source: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut h = DefaultHasher::new();
        source.hash(&mut h);
        request.language.profile.id.hash(&mut h);
        request.language.language_version.as_ref().map(|v| v.as_ref()).unwrap_or("").hash(&mut h);
        request.host_schema.content_hash().hash(&mut h);
        request.host_schema.abi_version.hash(&mut h);
        request.optimization.hash(&mut h);
        request.determinism.hash(&mut h);
        request.debug_info.hash(&mut h);
        ARTIFACT_FORMAT_VERSION.hash(&mut h);
        env!("CARGO_PKG_VERSION").hash(&mut h);
        for cap in &request.required_capabilities {
            cap.path.hash(&mut h);
        }
        for flag in &request.feature_flags {
            flag.hash(&mut h);
        }
        h.finish()
    }

    /// 按键查找；命中则克隆返回并增加 `hits`，否则增加 `misses`。
    pub fn get(&mut self, key: u64) -> Option<CompiledPackage> {
        match self.entries.get(&key) {
            Some(pkg) => {
                self.hits += 1;
                Some(pkg.clone())
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    /// 写入或覆盖指定键的编译包。
    pub fn insert(&mut self, key: u64, package: CompiledPackage) {
        self.entries.insert(key, package);
    }
}
