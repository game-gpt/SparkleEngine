//! 按时间取样。不推进播放时钟。

use spark_geometry::{Quat, Trs, Vec3};

use crate::clip::{AnimationClip, AnimationFrame};
use crate::pose::LocalPose;
use crate::skeleton::Skeleton;
use crate::track::{JointTrack, QuatKey, SkinnedAnimationClip, Vec3Key};

/// 把时间折进 `[0, duration)`。时长过小则返回 0。
pub fn wrap_time(time: f32, duration: f32) -> f32 {
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

/// 阶梯采样：取时间不大于 `time` 的最后一帧。帧须按时间升序。
pub fn sample_clip<T: Clone>(clip: &AnimationClip<T>, time: f32) -> Option<T> {
    let track = clip.tracks.first()?;
    sample_frames(&track.frames, wrap_time(time, clip.duration))
}

pub fn sample_frames<T: Clone>(frames: &[AnimationFrame<T>], time: f32) -> Option<T> {
    if frames.is_empty() {
        return None;
    }
    let mut chosen = &frames[0];
    for frame in frames {
        if frame.time <= time {
            chosen = frame;
        } else {
            break;
        }
    }
    Some(chosen.value.clone())
}

/// 在 `time`（秒）采样局部姿态。超出时长时循环。
pub fn sample_skinned_clip(
    skeleton: &Skeleton,
    clip: &SkinnedAnimationClip,
    time: f32,
) -> LocalPose {
    let mut pose = LocalPose::rest(skeleton);
    let t = wrap_time(time, clip.duration);
    for track in &clip.tracks {
        apply_joint_track(&mut pose, track, t);
    }
    pose
}

fn apply_joint_track(pose: &mut LocalPose, track: &JointTrack, time: f32) {
    let j = track.joint as usize;
    if j >= pose.locals.len() {
        return;
    }
    let rest = pose.locals[j];
    pose.locals[j] = Trs {
        translation: sample_vec3(&track.translations, time, rest.translation),
        rotation: sample_quat(&track.rotations, time, rest.rotation),
        scale: sample_vec3(&track.scales, time, rest.scale),
    };
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
