//! `.spkx` 可执行映像磁盘格式（`ExecutableImage` 的发布制品）。
//!
//! 装载后仍须用运行期 [`HostSchema`] 做哈希校验；字节码在解码时按槽位数再验证一次。

use std::sync::Arc;

use spark_vm::verify_bytecode_with_host;

use crate::{
    artifact::{ARTIFACT_FORMAT_VERSION, ExecutableImage},
    codec::{ArtifactIoError, Reader, SPKX_MAGIC, Writer, read_language, read_module, write_language, write_module},
    request::PackageId,
};

impl ExecutableImage {
    /// 编码为 `.spkx` 字节（不含运行状态）。
    pub fn to_spkx_bytes(&self) -> Result<Vec<u8>, ArtifactIoError> {
        let mut w = Writer::new();
        w.buf.extend_from_slice(SPKX_MAGIC);
        w.u32(self.format_version);
        w.u32(self.host_abi_version);
        w.u64(self.host_schema_hash);
        w.u32(self.host_slot_count);
        w.str(self.package.name.as_ref());
        w.str(self.package.version.as_ref());
        write_language(&mut w, &self.language);
        w.u32(self.lifecycle_exports.len() as u32);
        for e in &self.lifecycle_exports {
            w.str(e.as_ref());
        }
        write_module(&mut w, self.module())?;
        Ok(w.buf)
    }

    /// 从 `.spkx` 字节解码并再验证字节码。
    pub fn from_spkx_bytes(bytes: &[u8]) -> Result<Self, ArtifactIoError> {
        let mut r = Reader::new(bytes);
        let magic = r.take(4)?;
        if magic != SPKX_MAGIC {
            return Err(ArtifactIoError::BadMagic);
        }
        let format_version = r.u32()?;
        if format_version != ARTIFACT_FORMAT_VERSION {
            return Err(ArtifactIoError::UnsupportedFormat { version: format_version });
        }
        let host_abi_version = r.u32()?;
        let host_schema_hash = r.u64()?;
        let host_slot_count = r.u32()?;
        let package = PackageId::new(r.str()?, r.str()?);
        let language = read_language(&mut r)?;
        let life_n = r.u32()? as usize;
        let mut lifecycle_exports = Vec::with_capacity(life_n);
        for _ in 0..life_n {
            lifecycle_exports.push(Arc::<str>::from(r.str()?));
        }
        let module = read_module(&mut r)?;
        verify_bytecode_with_host(&module, host_slot_count)?;
        Ok(Self::from_decoded(
            format_version,
            package,
            language,
            host_schema_hash,
            host_abi_version,
            host_slot_count,
            lifecycle_exports,
            module,
        ))
    }

    /// 写入 `.spkx` 文件。
    pub fn write_spkx_file(&self, path: impl AsRef<std::path::Path>) -> Result<(), ArtifactIoError> {
        let bytes = self.to_spkx_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// 读取 `.spkx` 文件。
    pub fn read_spkx_file(path: impl AsRef<std::path::Path>) -> Result<Self, ArtifactIoError> {
        let bytes = std::fs::read(path)?;
        Self::from_spkx_bytes(&bytes)
    }
}
