//! 检查器演示用 Transform 字段（非 ECS 权威）。

use spark_widget::tree::WidgetTree;

/// 演示实体默认世界位置（与 Hierarchy 选中同步）。
pub fn demo_entity_position(entity_id: u64) -> (f32, f32, f32) {
    ((entity_id as f32 - 1.0) * 16.0, 0.0, 0.0)
}

/// Inspector Transform 文本框稳定键。
pub mod field_keys {
    /// Position X。
    pub const POS_X: &str = "inspector.transform.pos_x";
    /// Position Y。
    pub const POS_Y: &str = "inspector.transform.pos_y";
    /// Position Z。
    pub const POS_Z: &str = "inspector.transform.pos_z";
    /// Rotation X。
    pub const ROT_X: &str = "inspector.transform.rot_x";
    /// Rotation Y。
    pub const ROT_Y: &str = "inspector.transform.rot_y";
    /// Rotation Z。
    pub const ROT_Z: &str = "inspector.transform.rot_z";
    /// Scale X。
    pub const SCALE_X: &str = "inspector.transform.scale_x";
    /// Scale Y。
    pub const SCALE_Y: &str = "inspector.transform.scale_y";
    /// Scale Z。
    pub const SCALE_Z: &str = "inspector.transform.scale_z";
}

/// 可编辑的 Transform 快照。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformState {
    /// X 位置。
    pub pos_x: f32,
    /// Y 位置。
    pub pos_y: f32,
    /// Z 位置。
    pub pos_z: f32,
    /// X 旋转（度）。
    pub rot_x: f32,
    /// Y 旋转（度）。
    pub rot_y: f32,
    /// Z 旋转（度）。
    pub rot_z: f32,
    /// X 缩放。
    pub scale_x: f32,
    /// Y 缩放。
    pub scale_y: f32,
    /// Z 缩放。
    pub scale_z: f32,
}

impl Default for TransformState {
    fn default() -> Self {
        Self { pos_x: 0.0, pos_y: 0.0, pos_z: 0.0, rot_x: 0.0, rot_y: 0.0, rot_z: 0.0, scale_x: 1.0, scale_y: 1.0, scale_z: 1.0 }
    }
}

impl TransformState {
    /// 从演示实体 ID 生成默认 Transform。
    pub fn from_entity(entity_id: u64) -> Self {
        let (x, y, z) = demo_entity_position(entity_id);
        Self { pos_x: x, pos_y: y, pos_z: z, rot_x: 0.0, rot_y: 0.0, rot_z: 0.0, scale_x: 1.0, scale_y: 1.0, scale_z: 1.0 }
    }

    /// 从 Inspector 文本框读回并写进 `out`；任一字段解析成功则返回 `true`。
    pub fn read_from_tree(tree: &WidgetTree, scene_root: spark_widget::WidgetId, out: &mut Self) -> bool {
        let mut changed = false;
        if let Some(v) = read_field(tree, scene_root, field_keys::POS_X) {
            out.pos_x = v;
            changed = true;
        }
        if let Some(v) = read_field(tree, scene_root, field_keys::POS_Y) {
            out.pos_y = v;
            changed = true;
        }
        if let Some(v) = read_field(tree, scene_root, field_keys::POS_Z) {
            out.pos_z = v;
            changed = true;
        }
        if let Some(v) = read_field(tree, scene_root, field_keys::ROT_X) {
            out.rot_x = v;
            changed = true;
        }
        if let Some(v) = read_field(tree, scene_root, field_keys::ROT_Y) {
            out.rot_y = v;
            changed = true;
        }
        if let Some(v) = read_field(tree, scene_root, field_keys::ROT_Z) {
            out.rot_z = v;
            changed = true;
        }
        if let Some(v) = read_field(tree, scene_root, field_keys::SCALE_X) {
            out.scale_x = v;
            changed = true;
        }
        if let Some(v) = read_field(tree, scene_root, field_keys::SCALE_Y) {
            out.scale_y = v;
            changed = true;
        }
        if let Some(v) = read_field(tree, scene_root, field_keys::SCALE_Z) {
            out.scale_z = v;
            changed = true;
        }
        changed
    }
}

fn read_field(tree: &WidgetTree, root: spark_widget::WidgetId, key: &str) -> Option<f32> {
    let id = tree.find_by_key(root, key)?;
    let text = tree.node(id)?.content.text.as_deref()?;
    parse_scalar(text)
}

fn parse_scalar(raw: &str) -> Option<f32> {
    let trimmed = raw.trim().trim_end_matches('°');
    trimmed.parse().ok()
}
