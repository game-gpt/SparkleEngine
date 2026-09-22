//! 骨架与命名挂点。不含蒙皮网格提交。

use spark_geometry::{Mat4, Trs, Vec3};

/// 首切关节 palette 上限（与渲染 uniform 对齐）。
pub const MAX_JOINTS: usize = 64;

/// 骨架中的一个关节：拓扑、绑定逆矩阵与 rest 局部 TRS。
#[derive(Debug, Clone)]
pub struct Joint {
    /// 关节名，供 `find_joint` 与调试使用。
    pub name: String,
    /// 父关节下标。根为 `None`。
    pub parent: Option<u16>,
    /// 绑定姿态下「模型空间 → 关节局部」的逆矩阵，用于皮肤矩阵。
    pub inverse_bind: Mat4,
    /// 无动画时的局部 TRS（相对父关节）。
    pub rest_local: Trs,
}

/// 挂在某关节上的命名附着点（武器、特效原点等）。
#[derive(Debug, Clone)]
pub struct Socket {
    /// 挂点名，供 `find_socket` / `socket_world_*` 查找。
    pub name: String,
    /// 父关节在 `joints` 中的下标。
    pub parent_joint: u16,
    /// 相对父关节的局部 TRS。
    pub local: Trs,
}

/// 关节树与挂点集合。关节须父先于子存储。
#[derive(Debug, Clone, Default)]
pub struct Skeleton {
    /// 关节列表，下标即关节 ID；长度建议 ≤ [`MAX_JOINTS`]。
    pub joints: Vec<Joint>,
    /// 命名挂点，不计入关节 palette。
    pub sockets: Vec<Socket>,
}

impl Skeleton {
    /// 关节数量。
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// 按名查找关节下标；重名时返回首次匹配。
    pub fn find_joint(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }

    /// 按名查找挂点下标；重名时返回首次匹配。
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
