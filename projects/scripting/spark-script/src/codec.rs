//! 制品磁盘编解码共用原语（`.spko` / `.spkx`）。

use spark_gc::Value;
use spark_vm::{FuncProto, Module};

use crate::request::LanguageFrontend;

/// `.spkx` 魔数。
pub const SPKX_MAGIC: &[u8; 4] = b"SPKX";
/// `.spko` 魔数。
pub const SPKO_MAGIC: &[u8; 4] = b"SPKO";

/// 制品读写错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactIoError {
    Truncated,
    BadMagic,
    UnsupportedFormat { version: u32 },
    BadValueTag { tag: u8 },
    HandleConstForbidden,
    Utf8,
    Io,
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
            Self::Io => "spark.script.artifact.io",
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

impl From<std::io::Error> for ArtifactIoError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}

pub(crate) struct Writer {
    pub buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn bytes(&mut self, data: &[u8]) {
        self.u32(data.len() as u32);
        self.buf.extend_from_slice(data);
    }

    pub fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
}

pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remain(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], ArtifactIoError> {
        if self.remain() < n {
            return Err(ArtifactIoError::Truncated);
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn u8(&mut self) -> Result<u8, ArtifactIoError> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, ArtifactIoError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Result<u32, ArtifactIoError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u64(&mut self) -> Result<u64, ArtifactIoError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn f64(&mut self) -> Result<f64, ArtifactIoError> {
        Ok(f64::from_bits(self.u64()?))
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], ArtifactIoError> {
        let n = self.u32()? as usize;
        self.take(n)
    }

    pub fn str(&mut self) -> Result<String, ArtifactIoError> {
        let b = self.bytes()?;
        std::str::from_utf8(b)
            .map(|s| s.to_string())
            .map_err(|_| ArtifactIoError::Utf8)
    }
}

pub(crate) fn write_value(w: &mut Writer, v: &Value) -> Result<(), ArtifactIoError> {
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

pub(crate) fn read_value(r: &mut Reader<'_>) -> Result<Value, ArtifactIoError> {
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

pub(crate) fn write_module(w: &mut Writer, m: &Module) -> Result<(), ArtifactIoError> {
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

pub(crate) fn read_module(r: &mut Reader<'_>) -> Result<Module, ArtifactIoError> {
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

pub(crate) fn frontend_tag(f: LanguageFrontend) -> u8 {
    match f {
        LanguageFrontend::Valkyrie => 0,
        LanguageFrontend::Lua => 1,
        LanguageFrontend::Ruby => 2,
    }
}

pub(crate) fn frontend_from_tag(tag: u8) -> Result<LanguageFrontend, ArtifactIoError> {
    match tag {
        0 => Ok(LanguageFrontend::Valkyrie),
        1 => Ok(LanguageFrontend::Lua),
        2 => Ok(LanguageFrontend::Ruby),
        _ => Err(ArtifactIoError::BadValueTag { tag }),
    }
}

pub(crate) fn write_language(w: &mut Writer, language: &crate::request::LanguageProfile) {
    w.u8(frontend_tag(language.frontend));
    w.str(language.profile.as_str());
    match &language.language_version {
        Some(v) => {
            w.u8(1);
            w.str(v.as_ref());
        }
        None => w.u8(0),
    }
}

pub(crate) fn read_language(
    r: &mut Reader<'_>,
) -> Result<crate::request::LanguageProfile, ArtifactIoError> {
    use crate::request::{LanguageProfile, LanguageProfileId};
    let frontend = frontend_from_tag(r.u8()?)?;
    let profile = LanguageProfileId::new(r.str()?);
    let mut language = LanguageProfile::new(frontend, profile);
    if r.u8()? != 0 {
        language = language.with_language_version(r.str()?);
    }
    Ok(language)
}
