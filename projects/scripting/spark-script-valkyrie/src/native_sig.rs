//! 宿主函数参数与类型路径（供 [`spark_script::HostFunction`] 等复用）。

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_carries_name_and_type() {
        let p = NativeParam::new("id", "u32").with_docs("实体编号");
        assert_eq!(p.name.as_ref(), "id");
        assert_eq!(p.ty.as_str(), "u32");
        assert!(p.docs.is_some());
    }
}
