//! `.spkx` 可执行映像磁盘格式（`ExecutableImage` 的发布制品）。
//!
//! 手写小端布局，无第三方序列化依赖。装载后仍须用运行期 [`HostSchema`]
//! 做哈希校验；字节码在解码时按槽位数再验证一次。

use std::sync::Arc;

use spark_gc::Value;
use spark_vm::{verify_bytecode_with_host, FuncProto, Module};

use crate::artifact::{ExecutableImage, ARTIFACT_FORMAT_VERSION};
use crate::request::{LanguageFrontend, LanguageProfile, LanguageProfileId, PackageId};

/// 文件魔数。
pub const SPKX_MAGIC: &[u8; 4] = b"SPKX";

/// 制品读写错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactIoError {
    Truncated,
    BadMagic,
    UnsupportedFormat { version: u32 },
    BadValueTag { tag: u8 },
    HandleConstForbidden,
    Utf8,
    Verify(spark_vm::BytecodeVerifyError),
}

impl ArtifactIoError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Truncated => "spark.script.artifact.truncated",
            Self::BadMagic => "spark.script.artifact.bad_magic",
            Self::UnsupportedFormat { .. } => "spark.script.artifact.unsupported_format",
            Self::BadValueTag { .. } => "spark.script.artifact.bad_value_tag",
            Self::HandleConstForbidden => "spark.script.artifact.handle_const_forbidden",
            Self::Utf8 => "spark.script.artifact.utf8",
            Self::Verify(e) => e.code(),
        }
    }
}

impl std::fmt::Display for ArtifactIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ArtifactIoError {}

impl From<spark_vm::BytecodeVerifyError> for ArtifactIoError {
    fn from(value: spark_vm::BytecodeVerifyError) -> Self {
        Self::Verify(value)
    }
}

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn bytes(&mut self, data: &[u8]) {
        self.u32(data.len() as u32);
        self.buf.extend_from_slice(data);
    }

    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remain(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ArtifactIoError> {
        if self.remain() < n {
            return Err(ArtifactIoError::Truncated);
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, ArtifactIoError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ArtifactIoError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, ArtifactIoError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, ArtifactIoError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn f64(&mut self) -> Result<f64, ArtifactIoError> {
        Ok(f64::from_bits(self.u64()?))
    }

    fn bytes(&mut self) -> Result<&'a [u8], ArtifactIoError> {
        let n = self.u32()? as usize;
        self.take(n)
    }

    fn str(&mut self) -> Result<String, ArtifactIoError> {
        let b = self.bytes()?;
        std::str::from_utf8(b)
            .map(|s| s.to_string())
            .map_err(|_| ArtifactIoError::Utf8)
    }
}

fn write_value(w: &mut Writer, v: &Value) -> Result<(), ArtifactIoError> {
    match v {
        Value::Null => w.u8(0),
        Value::Bool(b) => {
            w.u8(1);
            w.u8(u8::from(*b));
        }
        Value::Number(n) => {
            w.u8(2);
            w.f64(*n);
        }
        Value::Entity(e) => {
            w.u8(3);
            w.u64(*e);
        }
        Value::Func(i) => {
            w.u8(4);
            w.u32(*i);
        }
        Value::Handle(_) => return Err(ArtifactIoError::HandleConstForbidden),
    }
    Ok(())
}

fn read_value(r: &mut Reader<'_>) -> Result<Value, ArtifactIoError> {
    match r.u8()? {
        0 => Ok(Value::Null),
        1 => Ok(Value::Bool(r.u8()? != 0)),
        2 => Ok(Value::Number(r.f64()?)),
        3 => Ok(Value::Entity(r.u64()?)),
        4 => Ok(Value::Func(r.u32()?)),
        tag => Err(ArtifactIoError::BadValueTag { tag }),
    }
}

fn write_func(w: &mut Writer, f: &FuncProto) -> Result<(), ArtifactIoError> {
    w.str(&f.name);
    w.u8(f.arity);
    w.u16(f.locals);
    w.bytes(&f.code);
    w.u32(f.consts.len() as u32);
    for c in &f.consts {
        write_value(w, c)?;
    }
    w.u32(f.const_names.len() as u32);
    for n in &f.const_names {
        w.str(n);
    }
    w.u32(f.strings.len() as u32);
    for s in &f.strings {
        w.str(s);
    }
    Ok(())
}

fn read_func(r: &mut Reader<'_>) -> Result<FuncProto, ArtifactIoError> {
    let name = r.str()?;
    let arity = r.u8()?;
    let locals = r.u16()?;
    let code = r.bytes()?.to_vec();
    let const_n = r.u32()? as usize;
    let mut consts = Vec::with_capacity(const_n);
    for _ in 0..const_n {
        consts.push(read_value(r)?);
    }
    let name_n = r.u32()? as usize;
    let mut const_names = Vec::with_capacity(name_n);
    for _ in 0..name_n {
        const_names.push(r.str()?);
    }
    let str_n = r.u32()? as usize;
    let mut strings = Vec::with_capacity(str_n);
    for _ in 0..str_n {
        strings.push(r.str()?);
    }
    Ok(FuncProto {
        name,
        arity,
        locals,
        code,
        consts,
        const_names,
        strings,
    })
}

fn write_module(w: &mut Writer, m: &Module) -> Result<(), ArtifactIoError> {
    w.u32(m.entry as u32);
    w.u32(m.native_names.len() as u32);
    for n in &m.native_names {
        w.str(n);
    }
    w.u32(m.functions.len() as u32);
    for f in &m.functions {
        write_func(w, f)?;
    }
    Ok(())
}

fn read_module(r: &mut Reader<'_>) -> Result<Module, ArtifactIoError> {
    let entry = r.u32()? as usize;
    let native_n = r.u32()? as usize;
    let mut native_names = Vec::with_capacity(native_n);
    for _ in 0..native_n {
        native_names.push(r.str()?);
    }
    let func_n = r.u32()? as usize;
    let mut functions = Vec::with_capacity(func_n);
    for _ in 0..func_n {
        functions.push(read_func(r)?);
    }
    Ok(Module {
        functions,
        entry,
        native_names,
    })
}

fn frontend_tag(f: LanguageFrontend) -> u8 {
    match f {
        LanguageFrontend::Valkyrie => 0,
        LanguageFrontend::Lua => 1,
        LanguageFrontend::Ruby => 2,
    }
}

fn frontend_from_tag(tag: u8) -> Result<LanguageFrontend, ArtifactIoError> {
    match tag {
        0 => Ok(LanguageFrontend::Valkyrie),
        1 => Ok(LanguageFrontend::Lua),
        2 => Ok(LanguageFrontend::Ruby),
        _ => Err(ArtifactIoError::BadValueTag { tag }),
    }
}

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
        w.u8(frontend_tag(self.language.frontend));
        w.str(self.language.profile.as_str());
        match &self.language.language_version {
            Some(v) => {
                w.u8(1);
                w.str(v.as_ref());
            }
            None => w.u8(0),
        }
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
            return Err(ArtifactIoError::UnsupportedFormat {
                version: format_version,
            });
        }
        let host_abi_version = r.u32()?;
        let host_schema_hash = r.u64()?;
        let host_slot_count = r.u32()?;
        let package = PackageId::new(r.str()?, r.str()?);
        let frontend = frontend_from_tag(r.u8()?)?;
        let profile = LanguageProfileId::new(r.str()?);
        let mut language = LanguageProfile::new(frontend, profile);
        if r.u8()? != 0 {
            language = language.with_language_version(r.str()?);
        }
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
        std::fs::write(path, bytes).map_err(|_| ArtifactIoError::Truncated)
    }

    /// 读取 `.spkx` 文件。
    pub fn read_spkx_file(path: impl AsRef<std::path::Path>) -> Result<Self, ArtifactIoError> {
        let bytes = std::fs::read(path).map_err(|_| ArtifactIoError::Truncated)?;
        Self::from_spkx_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{LinkedProgram, SparkObject};
    use crate::host_schema::{HostFunction, HostFunctionId, HostSchema};
    use crate::ScriptLanguage;
    use spark_script_valkyrie::NativeParam;
    use spark_vm::{FuncProto, Op};

    fn schema_with_print() -> HostSchema {
        let mut schema = HostSchema::new(1);
        schema.insert(
            HostFunction::new(HostFunctionId::new("host", "print", 1))
                .param(NativeParam::new("msg", "String"))
                .returns("Null"),
        );
        schema
    }

    fn sample_image() -> ExecutableImage {
        let mut f = FuncProto::new("__main", 0);
        let c = f.add_const_number(42.0);
        f.emit(Op::LoadConst);
        f.emit_u16(c);
        f.emit(Op::Return);
        let host = schema_with_print();
        let obj = SparkObject::from_legacy_module(
            PackageId::new("demo", "1"),
            LanguageProfile::default_for(ScriptLanguage::Valkyrie),
            &host,
            Module {
                functions: vec![f],
                entry: 0,
                native_names: vec!["print".into()],
            },
        );
        let linked = LinkedProgram::link_single(obj, &host).unwrap();
        ExecutableImage::verify(linked).unwrap()
    }

    #[test]
    fn spkx_roundtrip_preserves_module() {
        let image = sample_image();
        let bytes = image.to_spkx_bytes().unwrap();
        assert_eq!(&bytes[..4], SPKX_MAGIC);
        let loaded = ExecutableImage::from_spkx_bytes(&bytes).unwrap();
        assert_eq!(loaded.host_schema_hash, image.host_schema_hash);
        assert_eq!(loaded.host_slot_count, image.host_slot_count);
        assert_eq!(loaded.package.name.as_ref(), "demo");
        assert_eq!(loaded.module().entry, image.module().entry);
        assert_eq!(loaded.module().functions[0].code, image.module().functions[0].code);
        assert_eq!(
            loaded.module().functions[0].consts[0].as_number(),
            Some(42.0)
        );
    }

    #[test]
    fn bad_magic_rejected() {
        let err = ExecutableImage::from_spkx_bytes(b"XXXX").unwrap_err();
        assert!(matches!(err, ArtifactIoError::BadMagic));
    }
}
