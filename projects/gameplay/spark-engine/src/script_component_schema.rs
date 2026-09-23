//! 脚本可见组件列布局（绑定期登记；热路径按槽位 + 字段偏移访问）。

use std::sync::Arc;

/// 脚本列字段物理类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptFieldKind {
    /// 32 位浮点。
    F32,
    /// 32 位有符号整数。
    I32,
    /// 布尔（布局占 1 字节，列存储按 4 字节对齐）。
    Bool,
}

/// 单列内字段布局（稳定偏移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFieldLayout {
    /// 字段逻辑名。
    pub name: Arc<str>,
    /// 物理类型。
    pub kind: ScriptFieldKind,
    /// 相对组件结构体首地址的字节偏移。
    pub offset: u32,
}

/// 脚本组件列布局（游戏作者在绑定期提供）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptComponentLayout {
    /// 字段表（按偏移升序）。
    pub fields: Vec<ScriptFieldLayout>,
    /// 结构体字节大小（对齐后）。
    pub size: u32,
}

impl ScriptComponentLayout {
    /// 构造并校验字段偏移不重叠、落在 `size` 内。
    pub fn new(fields: Vec<ScriptFieldLayout>, size: u32) -> Result<Self, ScriptComponentLayoutError> {
        for field in &fields {
            let width = field_width(field.kind);
            if field.offset + width > size {
                return Err(ScriptComponentLayoutError::FieldOutOfBounds {
                    field: field.name.to_string(),
                    offset: field.offset,
                    size,
                });
            }
        }
        for (i, a) in fields.iter().enumerate() {
            for b in fields.iter().skip(i + 1) {
                if ranges_overlap(a, b) {
                    return Err(ScriptComponentLayoutError::OverlappingFields {
                        a: a.name.to_string(),
                        b: b.name.to_string(),
                    });
                }
            }
        }
        Ok(Self { fields, size })
    }

    /// 按字段名查布局。
    pub fn field(&self, name: &str) -> Option<&ScriptFieldLayout> {
        self.fields.iter().find(|f| f.name.as_ref() == name)
    }
}

/// 组件布局登记错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptComponentLayoutError {
    /// 字段区间超出组件 `size`。
    FieldOutOfBounds {
        /// 字段名。
        field: String,
        /// 声明偏移。
        offset: u32,
        /// 组件大小。
        size: u32,
    },
    /// 两字段内存区间重叠。
    OverlappingFields {
        /// 字段 A。
        a: String,
        /// 字段 B。
        b: String,
    },
}

impl std::fmt::Display for ScriptComponentLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FieldOutOfBounds { field, offset, size } => {
                write!(f, "spark.engine.component_layout.out_of_bounds:{field}@{offset}/size={size}")
            }
            Self::OverlappingFields { a, b } => write!(f, "spark.engine.component_layout.overlap:{a}/{b}"),
        }
    }
}

impl std::error::Error for ScriptComponentLayoutError {}

/// 流式构造 [`ScriptComponentLayout`]。
#[derive(Debug, Default)]
pub struct ScriptComponentLayoutBuilder {
    fields: Vec<ScriptFieldLayout>,
    size: u32,
}

impl ScriptComponentLayoutBuilder {
    /// 空构造器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加字段并自动推进偏移（按类型宽度对齐到 4 字节）。
    pub fn field(mut self, name: impl Into<Arc<str>>, kind: ScriptFieldKind) -> Self {
        let offset = align4(self.size);
        let width = field_width(kind);
        self.fields.push(ScriptFieldLayout { name: name.into(), kind, offset });
        self.size = offset + width;
        self
    }

    /// 指定结构体总大小（不得小于已追加字段尾偏移）。
    pub fn size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }

    /// 完成布局并校验。
    pub fn build(self) -> Result<ScriptComponentLayout, ScriptComponentLayoutError> {
        ScriptComponentLayout::new(self.fields, self.size)
    }
}

fn field_width(kind: ScriptFieldKind) -> u32 {
    match kind {
        ScriptFieldKind::F32 | ScriptFieldKind::I32 | ScriptFieldKind::Bool => 4,
    }
}

fn align4(offset: u32) -> u32 {
    (offset + 3) & !3
}

fn ranges_overlap(a: &ScriptFieldLayout, b: &ScriptFieldLayout) -> bool {
    let a_end = a.offset + field_width(a.kind);
    let b_end = b.offset + field_width(b.kind);
    a.offset < b_end && b.offset < a_end
}
