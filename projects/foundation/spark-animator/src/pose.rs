//! 局部姿态、前向运动学与皮肤矩阵。输出给渲染侧，不在这里提交 GPU。

use spark_geometry::{Mat4, Trs, Vec3};

use crate::skeleton::{MAX_JOINTS, Skeleton};

/// 每关节局部 TRS（与 `Skeleton::joints` 对齐）。
#[derive(Debug, Clone)]
pub struct LocalPose {
    /// 关节局部变换，下标与骨架关节一一对应。
    pub locals: Vec<Trs>,
}

impl LocalPose {
    /// 从骨架 rest 姿态拷贝一份局部 TRS。
    pub fn rest(skeleton: &Skeleton) -> Self {
        Self { locals: skeleton.joints.iter().map(|j| j.rest_local).collect() }
    }

    /// 当前姿态的关节数（等于 `locals.len()`）。
    pub fn joint_count(&self) -> usize {
        self.locals.len()
    }
}

/// 前向运动学：局部 TRS → 全局矩阵（父×子）。
///
/// # 不变式
/// 关节须按父先于子的拓扑顺序存储。本函数按索引升序累积，父下标必须 `< i`。
pub fn evaluate_pose(skeleton: &Skeleton, pose: &LocalPose) -> Vec<Mat4> {
    let n = skeleton.joint_count().min(pose.locals.len()).min(MAX_JOINTS);
    let mut globals: Vec<Mat4> = Vec::with_capacity(n);
    for i in 0..n {
        let local = pose.locals[i].to_mat4();
        let global = match skeleton.joints[i].parent {
            Some(p) => {
                let p = p as usize;
                debug_assert!(p < i, "关节父下标必须小于子下标");
                globals[p].mul(local)
            }
            None => local,
        };
        globals.push(global);
    }
    globals
}

/// `skin = global * inverse_bind`。
pub fn build_skin_palette(skeleton: &Skeleton, globals: &[Mat4]) -> Vec<Mat4> {
    let n = skeleton.joint_count().min(globals.len()).min(MAX_JOINTS);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(globals[i].mul(skeleton.joints[i].inverse_bind));
    }
    out
}

/// 挂点世界矩阵：`joint_global * socket.local`。
pub fn socket_world_matrix(skeleton: &Skeleton, globals: &[Mat4], socket_name: &str) -> Option<Mat4> {
    let idx = skeleton.find_socket(socket_name)?;
    let socket = &skeleton.sockets[idx];
    let j = socket.parent_joint as usize;
    let g = *globals.get(j)?;
    Some(g.mul(socket.local.to_mat4()))
}

/// 挂点世界位置。
pub fn socket_world_position(skeleton: &Skeleton, globals: &[Mat4], socket_name: &str) -> Option<Vec3> {
    socket_world_matrix(skeleton, globals, socket_name).map(|m| m.transform_point(Vec3::ZERO))
}
