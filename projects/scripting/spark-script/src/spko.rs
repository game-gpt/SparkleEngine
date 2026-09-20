//! `.spko` 可重定位目标磁盘格式（[`SparkObject`]）。
//!
//! 目标未绑定宿主槽位；装载后仍须经 [`LinkedProgram::link_single`] / [`link_many`]。

use std::sync::Arc;

use crate::artifact::{SparkObject, ARTIFACT_FORMAT_VERSION};
use crate::codec::{
    read_language, read_module, write_language, write_module, ArtifactIoError, Reader, Writer,
    SPKO_MAGIC,
};
use crate::request::PackageId;

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
        write_module(&mut w, &self.legacy_module)?;
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
            return Err(ArtifactIoError::UnsupportedFormat {
                version: format_version,
            });
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
        let legacy_module = read_module(&mut r)?;
        Ok(Self {
            format_version,
            compiler_version,
            package,
            language,
            host_schema_hash,
            host_abi_version,
            legacy_module,
            exports,
            imports,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{ExecutableImage, LinkedProgram};
    use crate::codec::SPKO_MAGIC;
    use crate::host_schema::{HostFunction, HostFunctionId, HostSchema};
    use crate::request::{LanguageProfile, PackageId};
    use crate::ScriptLanguage;
    use spark_script_valkyrie::NativeParam;
    use spark_vm::{FuncProto, Module, Op};

    fn schema_with_print() -> HostSchema {
        let mut schema = HostSchema::new(1);
        schema.insert(
            HostFunction::new(HostFunctionId::new("host", "print", 1))
                .param(NativeParam::new("msg", "String"))
                .returns("Null"),
        );
        schema
    }

    #[test]
    fn spko_roundtrip_then_link() {
        let mut f = FuncProto::new("__main", 0);
        let c = f.add_const_number(7.0);
        f.emit(Op::LoadConst);
        f.emit_u16(c);
        f.emit(Op::Return);
        let host = schema_with_print();
        let obj = SparkObject::from_legacy_module(
            PackageId::new("unit", "0.1"),
            LanguageProfile::default_for(ScriptLanguage::Valkyrie),
            &host,
            Module {
                functions: vec![f],
                entry: 0,
                native_names: vec!["print".into()],
            },
        );
        let bytes = obj.to_spko_bytes().unwrap();
        assert_eq!(&bytes[..4], SPKO_MAGIC);
        let loaded = SparkObject::from_spko_bytes(&bytes).unwrap();
        assert_eq!(loaded.package.name.as_ref(), "unit");
        assert_eq!(loaded.imports.len(), 1);
        let linked = LinkedProgram::link_single(loaded, &host).unwrap();
        let image = ExecutableImage::verify(linked).unwrap();
        let mut vm = spark_vm::Vm::new(image.clone_module());
        let v = vm.run(&mut spark_vm::StdHost).unwrap();
        assert_eq!(v.as_number(), Some(7.0));
    }

    #[test]
    fn spko_bad_magic() {
        let err = SparkObject::from_spko_bytes(b"NOPE").unwrap_err();
        assert!(matches!(err, ArtifactIoError::BadMagic));
    }
}
