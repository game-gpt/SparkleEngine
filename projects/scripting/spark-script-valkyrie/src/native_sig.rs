//! 宿主函数参数与类型路径（供 [`spark_script::HostFunction`] 等复用）。

use std::sync::Arc;

/// 类型引用（稳定字符串路径，后续可换成 interned id）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRef {
    /// 类型路径文本，例如 `i32` 或宿主自定义限定名；不做语法校验。
    pub path: Arc<str>,
}

impl TypeRef {
    /// 用任意可转成 `Arc<str>` 的路径构造。
    pub fn new(path: impl Into<Arc<str>>) -> Self {
        Self { path: path.into() }
    }

    /// 借用路径字符串。
    pub fn as_str(&self) -> &str {
        &self.path
    }
}

impl From<&str> for TypeRef {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// 原生/宿主函数的单个形参描述（元数据，不参与运行时压栈）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeParam {
    /// 形参名（脚本侧展示/绑定用）。
    pub name: Arc<str>,
    /// 形参类型路径。
    pub ty: TypeRef,
    /// 可选文档字符串；`None` 表示无说明。
    pub docs: Option<Arc<str>>,
}

impl NativeParam {
    /// 构造无名文档的形参。
    pub fn new(name: impl Into<Arc<str>>, ty: impl Into<TypeRef>) -> Self {
        Self { name: name.into(), ty: ty.into(), docs: None }
    }

    /// 附加文档说明后返回 `self`。
    pub fn with_docs(mut self, docs: impl Into<Arc<str>>) -> Self {
        self.docs = Some(docs.into());
        self
    }
}
