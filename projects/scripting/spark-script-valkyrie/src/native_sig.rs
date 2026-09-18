//! 宿主原生函数的完整类型描述。
//!
//! 旧入口只传 `&[&str]` 函数名，无法支撑补全与类型检查。
//! 内容 DSL / 编辑器应使用本模块的签名契约。

use std::sync::Arc;

/// 类型引用（稳定字符串路径，后续可换成 interned id）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRef {
    pub path: Arc<str>,
}

impl TypeRef {
    pub fn new(path: impl Into<Arc<str>>) -> Self {
        Self { path: path.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }
}

impl From<&str> for TypeRef {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// 单个参数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeParam {
    pub name: Arc<str>,
    pub ty: TypeRef,
    pub docs: Option<Arc<str>>,
}

impl NativeParam {
    pub fn new(name: impl Into<Arc<str>>, ty: impl Into<TypeRef>) -> Self {
        Self {
            name: name.into(),
            ty: ty.into(),
            docs: None,
        }
    }

    pub fn with_docs(mut self, docs: impl Into<Arc<str>>) -> Self {
        self.docs = Some(docs.into());
        self
    }
}

/// 宿主原生函数签名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSignature {
    pub name: Arc<str>,
    pub params: Vec<NativeParam>,
    pub return_ty: Option<TypeRef>,
    pub docs: Option<Arc<str>>,
}

impl NativeSignature {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            return_ty: None,
            docs: None,
        }
    }

    pub fn param(mut self, param: NativeParam) -> Self {
        self.params.push(param);
        self
    }

    pub fn returns(mut self, ty: impl Into<TypeRef>) -> Self {
        self.return_ty = Some(ty.into());
        self
    }

    pub fn with_docs(mut self, docs: impl Into<Arc<str>>) -> Self {
        self.docs = Some(docs.into());
        self
    }
}

/// 一组宿主绑定（供编译器 / 编辑器共享）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeRegistry {
    pub signatures: Vec<NativeSignature>,
}

impl NativeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, sig: NativeSignature) {
        if let Some(existing) = self.signatures.iter_mut().find(|s| s.name == sig.name) {
            *existing = sig;
        } else {
            self.signatures.push(sig);
        }
    }

    pub fn get(&self, name: &str) -> Option<&NativeSignature> {
        self.signatures.iter().find(|s| s.name.as_ref() == name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.signatures.iter().map(|s| s.name.as_ref()).collect()
    }

    /// 兼容旧编译入口：仅取函数名列表。
    pub fn name_list(&self) -> Vec<&str> {
        self.names()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_names_and_params() {
        let mut reg = NativeRegistry::new();
        reg.insert(
            NativeSignature::new("register_block")
                .with_docs("装载器内部用，非作者 DSL")
                .param(NativeParam::new("id", "u32"))
                .param(NativeParam::new("key", "String"))
                .returns("Null"),
        );
        let sig = reg.get("register_block").unwrap();
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].name.as_ref(), "id");
        assert_eq!(reg.names(), vec!["register_block"]);
    }
}
