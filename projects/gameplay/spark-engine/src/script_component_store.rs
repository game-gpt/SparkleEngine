//! 脚本组件列存储：绑定期布局 + 按实体位模式索引，供列直写 dispatch 读写。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::command_apply::ComponentDescriptorId;
use crate::script_component_schema::{ScriptComponentLayout, ScriptFieldKind};

/// 脚本组件列数据（与 ECS 实体解耦存储；提交期与命令缓冲协同）。
#[derive(Clone)]
pub struct ScriptComponentStore {
    inner: Arc<Mutex<ScriptComponentStoreInner>>,
}

#[derive(Debug, Default)]
struct ScriptComponentStoreInner {
    rows: HashMap<ComponentDescriptorId, HashMap<u64, Vec<u8>>>,
}

impl ScriptComponentStore {
    /// 空存储。
    pub fn new() -> Self {
        Self::default()
    }

    /// 确保实体在槽位上有布局大小的行（不存在则填零）。
    pub fn ensure_row(&mut self, slot: ComponentDescriptorId, entity_bits: u64, layout: &ScriptComponentLayout) {
        let mut inner = self.inner.lock().expect("script component store poisoned");
        let bucket = inner.rows.entry(slot).or_default();
        let row = bucket.entry(entity_bits).or_insert_with(|| vec![0u8; layout.size as usize]);
        if row.len() < layout.size as usize {
            row.resize(layout.size as usize, 0);
        }
    }

    /// 移除实体在某槽位的行。
    pub fn remove_row(&mut self, slot: ComponentDescriptorId, entity_bits: u64) {
        if let Some(bucket) = self.inner.lock().expect("script component store poisoned").rows.get_mut(&slot) {
            bucket.remove(&entity_bits);
        }
    }

    /// 实体销毁时清除全部槽位行。
    pub fn remove_entity(&mut self, entity_bits: u64) {
        for bucket in self.inner.lock().expect("script component store poisoned").rows.values_mut() {
            bucket.remove(&entity_bits);
        }
    }

    /// 读取 `field_index` 处 `f32` 字段。
    pub fn read_f32(
        &self,
        slot: ComponentDescriptorId,
        layout: &ScriptComponentLayout,
        entity_bits: u64,
        field_index: usize,
    ) -> Option<f32> {
        let field = layout.fields.get(field_index)?;
        if field.kind != ScriptFieldKind::F32 {
            return None;
        }
        let inner = self.inner.lock().expect("script component store poisoned");
        let row = inner.rows.get(&slot)?.get(&entity_bits)?;
        read_pod::<f32>(row, field.offset)
    }

    /// 写入 `field_index` 处 `f32` 字段；行不存在则失败。
    pub fn write_f32(
        &mut self,
        slot: ComponentDescriptorId,
        layout: &ScriptComponentLayout,
        entity_bits: u64,
        field_index: usize,
        value: f32,
    ) -> bool {
        let Some(field) = layout.fields.get(field_index) else {
            return false;
        };
        if field.kind != ScriptFieldKind::F32 {
            return false;
        }
        let mut inner = self.inner.lock().expect("script component store poisoned");
        let Some(row) = inner.rows.get_mut(&slot).and_then(|b| b.get_mut(&entity_bits)) else {
            return false;
        };
        write_pod(row, field.offset, value)
    }

    /// 读取 `field_index` 处 `i32` 字段。
    pub fn read_i32(
        &self,
        slot: ComponentDescriptorId,
        layout: &ScriptComponentLayout,
        entity_bits: u64,
        field_index: usize,
    ) -> Option<i32> {
        let field = layout.fields.get(field_index)?;
        if field.kind != ScriptFieldKind::I32 {
            return None;
        }
        let inner = self.inner.lock().expect("script component store poisoned");
        let row = inner.rows.get(&slot)?.get(&entity_bits)?;
        read_pod::<i32>(row, field.offset)
    }

    /// 写入 `field_index` 处 `i32` 字段；行不存在则失败。
    pub fn write_i32(
        &mut self,
        slot: ComponentDescriptorId,
        layout: &ScriptComponentLayout,
        entity_bits: u64,
        field_index: usize,
        value: i32,
    ) -> bool {
        let Some(field) = layout.fields.get(field_index) else {
            return false;
        };
        if field.kind != ScriptFieldKind::I32 {
            return false;
        }
        let mut inner = self.inner.lock().expect("script component store poisoned");
        let Some(row) = inner.rows.get_mut(&slot).and_then(|b| b.get_mut(&entity_bits)) else {
            return false;
        };
        write_pod(row, field.offset, value)
    }

    /// 读取 `field_index` 处 `bool` 字段（布局占 4 字节，非零为 true）。
    pub fn read_bool(
        &self,
        slot: ComponentDescriptorId,
        layout: &ScriptComponentLayout,
        entity_bits: u64,
        field_index: usize,
    ) -> Option<bool> {
        let field = layout.fields.get(field_index)?;
        if field.kind != ScriptFieldKind::Bool {
            return None;
        }
        let inner = self.inner.lock().expect("script component store poisoned");
        let row = inner.rows.get(&slot)?.get(&entity_bits)?;
        read_pod::<u32>(row, field.offset).map(|v| v != 0)
    }

    /// 写入 `field_index` 处 `bool` 字段；行不存在则失败。
    pub fn write_bool(
        &mut self,
        slot: ComponentDescriptorId,
        layout: &ScriptComponentLayout,
        entity_bits: u64,
        field_index: usize,
        value: bool,
    ) -> bool {
        let Some(field) = layout.fields.get(field_index) else {
            return false;
        };
        if field.kind != ScriptFieldKind::Bool {
            return false;
        }
        let mut inner = self.inner.lock().expect("script component store poisoned");
        let Some(row) = inner.rows.get_mut(&slot).and_then(|b| b.get_mut(&entity_bits)) else {
            return false;
        };
        write_pod(row, field.offset, if value { 1u32 } else { 0u32 })
    }
}

impl Default for ScriptComponentStore {
    fn default() -> Self {
        Self { inner: Arc::new(Mutex::new(ScriptComponentStoreInner::default())) }
    }
}

fn read_pod<T: Pod>(row: &[u8], offset: u32) -> Option<T> {
    let size = std::mem::size_of::<T>();
    let start = offset as usize;
    let end = start + size;
    if end > row.len() {
        return None;
    }
    // SAFETY: 布局绑定期已校验偏移与宽度；此处按字段类型宽度读取。
    let bytes = row.get(start..end)?;
    Some(unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const T) })
}

fn write_pod<T: Pod>(row: &mut [u8], offset: u32, value: T) -> bool {
    let size = std::mem::size_of::<T>();
    let start = offset as usize;
    let end = start + size;
    if end > row.len() {
        return false;
    }
    // SAFETY: 布局绑定期已校验偏移与宽度；此处按字段类型宽度写入。
    unsafe {
        std::ptr::write_unaligned(row.as_mut_ptr().add(start) as *mut T, value);
    }
    true
}

trait Pod {}

impl Pod for f32 {}
impl Pod for i32 {}
impl Pod for u32 {}
