//! 骨架与命名挂点。不含蒙皮网格提交。

use spark_geometry::{Mat4, Trs, Vec3};

/// 首切关节 palette 上限（与渲染 uniform 对齐）。
pub const MAX_JOINTS: usize = 64;

#[derive(Debug, Clone)]
pub struct Joint {
    pub name: String,
    /// 父关节下标。根为 `None`。
    pub parent: Option<u16>,
    pub inverse_bind: Mat4,
    pub rest_local: Trs,
}

#[derive(Debug, Clone)]
pub struct Socket {
    pub name: String,
    pub parent_joint: u16,
    pub local: Trs,
}

#[derive(Debug, Clone, Default)]
pub struct Skeleton {
    pub joints: Vec<Joint>,
    pub sockets: Vec<Socket>,
}

impl Skeleton {
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    pub fn find_joint(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }

    pub fn find_socket(&self, name: &str) -> Option<usize> {
        self.sockets.iter().position(|s| s.name == name)
    }

    /// 两骨竖直链：`root`(0) → `child`(1)，子骨长 `length`。挂点 `tip` 在子骨末端。
    pub fn two_bone_chain(length: f32) -> Self {
        let len = length.max(1e-3);
        let root = Joint { name: "root".into(), parent: None, inverse_bind: Mat4::IDENTITY, rest_local: Trs::IDENTITY };
        let child_rest = Trs::from_translation(Vec3::new(0.0, len, 0.0));
        let child =
            Joint { name: "child".into(), parent: Some(0), inverse_bind: Mat4::translation(Vec3::new(0.0, -len, 0.0)), rest_local: child_rest };
        let tip = Socket { name: "tip".into(), parent_joint: 1, local: Trs::from_translation(Vec3::new(0.0, len, 0.0)) };
        Self { joints: vec![root, child], sockets: vec![tip] }
    }
}
