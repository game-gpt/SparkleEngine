//! `.spko` 可重定位目标磁盘格式（[`SparkObject`]）。
//!
//! 目标未绑定宿主槽位；装载后仍须经 [`LinkedProgram::link_single`] / [`link_many`]。

use std::sync::Arc;

use crate::{
    artifact::{ARTIFACT_FORMAT_VERSION, SparkObject},
    codec::{ArtifactIoError, Reader, SPKO_MAGIC, Writer, read_language, read_module, write_language, write_module},
    request::PackageId,
};

impl SparkObject {
    /// 编码为 `.spko` 字节。
    pub fn to_spko_bytes(&self) -> Result<Vec<u8>, ArtifactIoError> {
        let mut w = Writer::new();
        w.buf.extend_from_slice(SPKO_MAGIC);
        w.u32(self.format_version);
        w.str(self.compiler_version.as_ref());
        w.u32(self.host_abi_version);
        w.u64(self.host_schema_hash);
        w.str(self.package.name.as_ref());
        w.str(self.package.version.as_ref());
        write_language(&mut w, &self.language);
        w.u32(self.exports.len() as u32);
        for e in &self.exports {
            w.str(e.as_ref());
        }
        w.u32(self.imports.len() as u32);
        for i in &self.imports {
            w.str(i.as_ref());
        }
        write_module(&mut w, &self.module)?;
        Ok(w.buf)
    }

    /// 从 `.spko` 字节解码（不做宿主绑定）。
    pub fn from_spko_bytes(bytes: &[u8]) -> Result<Self, ArtifactIoError> {
        let mut r = Reader::new(bytes);
        let magic = r.take(4)?;
        if magic != SPKO_MAGIC {
            return Err(ArtifactIoError::BadMagic);
        }
        let format_version = r.u32()?;
        if format_version != ARTIFACT_FORMAT_VERSION {
            return Err(ArtifactIoError::UnsupportedFormat { version: format_version });
        }
        let compiler_version = Arc::<str>::from(r.str()?);
        let host_abi_version = r.u32()?;
        let host_schema_hash = r.u64()?;
        let package = PackageId::new(r.str()?, r.str()?);
        let language = read_language(&mut r)?;
        let export_n = r.u32()? as usize;
        let mut exports = Vec::with_capacity(export_n);
        for _ in 0..export_n {
            exports.push(Arc::<str>::from(r.str()?));
        }
        let import_n = r.u32()? as usize;
        let mut imports = Vec::with_capacity(import_n);
        for _ in 0..import_n {
            imports.push(Arc::<str>::from(r.str()?));
        }
        let module = read_module(&mut r)?;
        Ok(Self { format_version, compiler_version, package, language, host_schema_hash, host_abi_version, module, exports, imports })
    }

    /// 写入 `.spko` 文件。
    pub fn write_spko_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), ArtifactIoError> {
        let bytes = self.to_spko_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// 读取 `.spko` 文件。
    pub fn read_spko_file(path: impl AsRef<std::path::Path>) -> Result<Self, ArtifactIoError> {
        let bytes = std::fs::read(path)?;
        Self::from_spko_bytes(&bytes)
    }
}
