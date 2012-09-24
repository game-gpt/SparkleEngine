//! 动画剪辑采样。

use spark_geometry::{Quat, Trs, Vec3};

use crate::pose::LocalPose;
use crate::skeleton::Skeleton;

#[derive(Debug, Clone, Copy)]
pub struct Vec3Key {
    pub time: f32,
    pub value: Vec3,
}

#[derive(Debug, Clone, Copy)]
pub struct QuatKey {
    pub time: f32,
    pub value: Quat,
}

/// 单关节的 TRS 轨道（缺省通道保持 rest）。
#[derive(Debug, Clone, Default)]
pub struct JointTrack {
    pub joint: u16,
    pub translations: Vec<Vec3Key>,
    pub rotations: Vec<QuatKey>,
    pub scales: Vec<Vec3Key>,
}

#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<JointTrack>,
}

fn wrap_time(time: f32, duration: f32) -> f32 {
    if duration <= 1e-8 {
        return 0.0;
    }
    if time < 0.0 {
        let mut t = time % duration;
        if t < 0.0 {
            t += duration;
        }
        return t;
    }
    if time <= duration {
        return time;
    }
    time % duration
}

fn sample_vec3(keys: &[Vec3Key], time: f32, fallback: Vec3) -> Vec3 {
    if keys.is_empty() {
        return fallback;
    }
    if time <= keys[0].time {
        return keys[0].value;
    }
    let last = keys.len() - 1;
    if time >= keys[last].time {
        return keys[last].value;
    }
    for i in 0..last {
        let a = &keys[i];
        let b = &keys[i + 1];
        if time >= a.time && time <= b.time {
            let span = (b.time - a.time).max(1e-8);
            let t = (time - a.time) / span;
            return a.value + (b.value - a.value) * t;
        }
    }
    fallback
}

fn sample_quat(keys: &[QuatKey], time: f32, fallback: Quat) -> Quat {
    if keys.is_empty() {
        return fallback;
    }
    if time <= keys[0].time {
        return keys[0].value;
    }
    let last = keys.len() - 1;
    if time >= keys[last].time {
        return keys[last].value;
    }
    for i in 0..last {
        let a = &keys[i];
        let b = &keys[i + 1];
        if time >= a.time && time <= b.time {
            let span = (b.time - a.time).max(1e-8);
            let t = (time - a.time) / span;
            return a.value.slerp(b.value, t);
        }
    }
    fallback
}

/// 在 `time`（秒，超出则按 `duration` 循环）采样局部姿态。
pub fn sample_clip(skeleton: &Skeleton, clip: &AnimationClip, time: f32) -> LocalPose {
    let mut pose = LocalPose::rest(skeleton);
    let t = wrap_time(time, clip.duration);
    for track in &clip.tracks {
        let j = track.joint as usize;
        if j >= pose.locals.len() {
            continue;
        }
        let rest = pose.locals[j];
        pose.locals[j] = Trs {
            translation: sample_vec3(&track.translations, t, rest.translation),
            rotation: sample_quat(&track.rotations, t, rest.rotation),
            scale: sample_vec3(&track.scales, t, rest.scale),
        };
    }
    pose
}
