//! Spark 骨架、挂点与动画采样（CPU）。
//!
//! **不含** GPU 上传、glTF I/O、游戏角色/装备语义。

mod blend;
mod clip;
mod pose;
mod skeleton;

pub use blend::{blend_local_poses, blend_masked};
pub use clip::{sample_clip, AnimationChannel, AnimationClip, JointTrack, Vec3Key, QuatKey};
pub use pose::{
    build_skin_palette, evaluate_pose, socket_world_matrix, socket_world_position, LocalPose,
};
pub use skeleton::{Joint, Skeleton, Socket, MAX_JOINTS};
pub use spark_geometry::{Mat4, Quat, Trs, Vec3};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_bone_rest_palette_is_identity_like() {
        let sk = Skeleton::two_bone_chain(1.0);
        let pose = LocalPose::rest(&sk);
        let globals = evaluate_pose(&sk, &pose);
        let palette = build_skin_palette(&sk, &globals);
        assert_eq!(palette.len(), 2);
        // rest * inverse_bind ≈ I
        for m in &palette {
            for i in 0..16 {
                let expected = if i % 5 == 0 { 1.0 } else { 0.0 };
                assert!(
                    (m.cols[i] - expected).abs() < 1e-4,
                    "palette not identity at {i}: {}",
                    m.cols[i]
                );
            }
        }
    }

    #[test]
    fn socket_follows_parent_joint() {
        let sk = Skeleton::two_bone_chain(1.0);
        let mut pose = LocalPose::rest(&sk);
        pose.locals[0].translation = Vec3::new(0.0, 2.0, 0.0);
        let globals = evaluate_pose(&sk, &pose);
        let world = socket_world_matrix(&sk, &globals, "tip").unwrap();
        let p = world.transform_point(Vec3::ZERO);
        // root@y=2 → child@+1 → tip@+1 ⇒ y=4
        assert!((p.y - 4.0).abs() < 1e-3, "got {}", p.y);
    }

    #[test]
    fn sample_clip_rotates_root() {
        let sk = Skeleton::two_bone_chain(1.0);
        let clip = AnimationClip {
            name: "spin".into(),
            duration: 1.0,
            tracks: vec![JointTrack {
                joint: 0,
                translations: vec![],
                rotations: vec![
                    QuatKey {
                        time: 0.0,
                        value: Quat::IDENTITY,
                    },
                    QuatKey {
                        time: 1.0,
                        value: Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2),
                    },
                ],
                scales: vec![],
            }],
        };
        let pose = sample_clip(&sk, &clip, 1.0);
        let tip = pose.locals[0]
            .rotation
            .rotate_vec3(Vec3::X);
        assert!((tip.z + 1.0).abs() < 1e-3, "got {:?}", tip);
    }
}
