//! ECS 接线。组件只保存播放器或控制器，系统只推进时间。

use std::collections::HashMap;

use spark_ecs::{Schedule, World};

use crate::clip::AnimationClip;
use crate::controller::Animator;
use crate::player::AnimationPlayer;

/// 本帧动画步长（秒）。由宿主写入，系统不读窗口或渲染器。
#[derive(Debug, Clone, Copy)]
pub struct AnimationDelta {
    pub dt: f32,
}

/// 按名字存放的剪辑库。
#[derive(Debug, Clone)]
pub struct ClipLibrary<T> {
    clips: HashMap<String, AnimationClip<T>>,
}

impl<T> Default for ClipLibrary<T> {
    fn default() -> Self {
        Self {
            clips: HashMap::new(),
        }
    }
}

impl<T> ClipLibrary<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, clip: AnimationClip<T>) {
        self.clips.insert(clip.name.clone(), clip);
    }

    pub fn get(&self, name: &str) -> Option<&AnimationClip<T>> {
        self.clips.get(name)
    }

    pub fn durations(&self) -> HashMap<String, f32> {
        self.clips
            .iter()
            .map(|(name, clip)| (name.clone(), clip.duration))
            .collect()
    }
}

/// 单段播放。没有状态机。
#[derive(Debug, Clone)]
pub struct AnimationPlayerComponent<T: Send + Sync + 'static> {
    pub clip: String,
    pub player: AnimationPlayer<T>,
}

/// 状态机播放。`controller` 决定当前剪辑名。
#[derive(Debug, Clone)]
pub struct AnimatorComponent<T: Send + Sync + 'static> {
    pub animator: Animator<T>,
}

pub fn install_player_system<T>(schedule: &mut Schedule)
where
    T: Send + Sync + 'static,
{
    schedule.add_fn("animation_player", tick_players::<T>);
}

pub fn install_animator_system<T>(schedule: &mut Schedule)
where
    T: Send + Sync + 'static,
{
    schedule.add_fn("animator", tick_animators::<T>);
}

pub fn tick_players<T>(world: &mut World)
where
    T: Send + Sync + 'static,
{
    let dt = delta(world);
    let durations = durations::<T>(world);
    world.for_each_mut::<AnimationPlayerComponent<T>>(|_, component| {
        let duration = durations.get(&component.clip).copied().unwrap_or(0.0);
        component.player.tick(dt, duration);
    });
}

pub fn tick_animators<T>(world: &mut World)
where
    T: Send + Sync + 'static,
{
    let dt = delta(world);
    let durations = durations::<T>(world);
    world.for_each_mut::<AnimatorComponent<T>>(|_, component| {
        component.animator.tick(dt, &durations);
    });
}

fn delta(world: &World) -> f32 {
    world
        .resources
        .get::<AnimationDelta>()
        .map(|delta| delta.dt)
        .unwrap_or(0.0)
}

fn durations<T: Send + Sync + 'static>(world: &World) -> HashMap<String, f32> {
    world
        .resources
        .get::<ClipLibrary<T>>()
        .map(|library| library.durations())
        .unwrap_or_default()
}
