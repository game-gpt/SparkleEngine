//! 根据参数和过渡决定当前播哪一段。不绘制。

use std::{collections::HashMap, marker::PhantomData};

use crate::{
    parameter::ParameterValue,
    player::AnimationPlayer,
    state::AnimatorState,
    transition::{AnimatorCondition, AnimatorTransition},
};

/// 状态、过渡和参数的描述。采样值类型不在这里。
#[derive(Debug, Clone, Default)]
pub struct AnimatorController {
    states: Vec<AnimatorState>,
    transitions: Vec<AnimatorTransition>,
    parameters: HashMap<String, ParameterValue>,
    default_state: String,
}

impl AnimatorController {
    /// 空图控制器，进入态为 `default_state`（可稍后再 `add_state`）。
    pub fn new(default_state: impl Into<String>) -> Self {
        Self { default_state: default_state.into(), ..Self::default() }
    }

    /// 注册一个状态节点。
    pub fn add_state(&mut self, state: AnimatorState) -> &mut Self {
        self.states.push(state);
        self
    }

    /// 注册一条边；条件全满足时才会从 `from` 切到 `to`。
    pub fn add_transition(&mut self, transition: AnimatorTransition) -> &mut Self {
        self.transitions.push(transition);
        self
    }

    /// 写入或覆盖命名参数的初值 / 当前值。
    pub fn set_parameter(&mut self, name: impl Into<String>, value: ParameterValue) -> &mut Self {
        self.parameters.insert(name.into(), value);
        self
    }

    /// 按名取状态定义。
    pub fn state(&self, name: &str) -> Option<&AnimatorState> {
        self.states.iter().find(|state| state.name == name)
    }

    /// 默认进入态名称（构造 [`Animator`] 时的起点）。
    pub fn default_state(&self) -> &str {
        &self.default_state
    }
}

#[derive(Debug, Clone)]
struct ActiveTransition {
    to: String,
    elapsed: f32,
    duration: f32,
}

/// 运行时控制器。`T` 与被播放剪辑的采样值一致。
#[derive(Debug, Clone)]
pub struct Animator<T> {
    controller: AnimatorController,
    player: AnimationPlayer<T>,
    current: String,
    transition: Option<ActiveTransition>,
    _sample: PhantomData<T>,
}

impl<T> Animator<T> {
    /// 从控制器描述构造，并进入 `default_state`。
    pub fn new(controller: AnimatorController) -> Self {
        let current = controller.default_state().to_string();
        let speed = controller.state(&current).map(|state| state.speed).unwrap_or(1.0);
        let mut player = AnimationPlayer::new();
        player.speed = speed;
        Self { controller, player, current, transition: None, _sample: PhantomData }
    }

    /// 当前状态名。
    pub fn current_state(&self) -> &str {
        &self.current
    }

    /// 只读访问内部播放时钟。
    pub fn player(&self) -> &AnimationPlayer<T> {
        &self.player
    }

    /// 可变访问内部播放时钟（改速度、强制采样等）。
    pub fn player_mut(&mut self) -> &mut AnimationPlayer<T> {
        &mut self.player
    }

    /// 过渡进度。没有过渡时为 0。
    pub fn transition_weight(&self) -> f32 {
        self.transition
            .as_ref()
            .map(|transition| if transition.duration <= 1e-8 { 1.0 } else { (transition.elapsed / transition.duration).clamp(0.0, 1.0) })
            .unwrap_or(0.0)
    }

    /// 设置布尔参数（供 `BoolEquals` 条件读取）。
    pub fn set_bool(&mut self, name: impl Into<String>, value: bool) {
        self.controller.parameters.insert(name.into(), ParameterValue::Bool(value));
    }

    /// 设置浮点参数（供 `FloatGreater` 条件读取）。
    pub fn set_float(&mut self, name: impl Into<String>, value: f32) {
        self.controller.parameters.insert(name.into(), ParameterValue::Float(value));
    }

    /// 拉高触发器；匹配过渡成功后会被清回 `false`。
    pub fn set_trigger(&mut self, name: impl Into<String>) {
        self.controller.parameters.insert(name.into(), ParameterValue::Trigger(true));
    }

    /// 推进播放，并在条件满足时开始或完成过渡。
    ///
    /// `durations` 的键是剪辑名。
    pub fn tick(&mut self, dt: f32, durations: &HashMap<String, f32>) {
        let dt = dt.max(0.0);
        if let Some(transition) = self.transition.as_mut() {
            transition.elapsed += dt;
            if transition.elapsed >= transition.duration {
                let to = transition.to.clone();
                self.transition = None;
                self.enter(&to);
            }
        }
        else if let Some(index) = self.find_transition() {
            let to = self.controller.transitions[index].to.clone();
            let duration = self.controller.transitions[index].duration;
            self.consume_triggers(index);
            if duration <= 1e-8 {
                self.enter(&to);
            }
            else {
                self.transition = Some(ActiveTransition { to, elapsed: dt, duration });
                if self.transition.as_ref().is_some_and(|transition| transition.elapsed >= transition.duration) {
                    let to = self.transition.take().unwrap().to;
                    self.enter(&to);
                }
            }
        }
        let clip_name = self.controller.state(&self.current).map(|state| state.clip.clone()).unwrap_or_default();
        let duration = durations.get(&clip_name).copied().unwrap_or(0.0);
        self.player.tick(dt, duration);
    }

    fn enter(&mut self, state_name: &str) {
        self.current = state_name.to_string();
        let speed = self.controller.state(state_name).map(|state| state.speed).unwrap_or(1.0);
        self.player.reset();
        self.player.speed = speed;
        self.player.play();
    }

    fn find_transition(&self) -> Option<usize> {
        self.controller.transitions.iter().position(|transition| {
            transition.from == self.current && transition.conditions.iter().all(|condition| self.condition_met(condition))
        })
    }

    fn condition_met(&self, condition: &AnimatorCondition) -> bool {
        match condition {
            AnimatorCondition::BoolEquals { parameter, value } => {
                matches!(
                    self.controller.parameters.get(parameter),
                    Some(ParameterValue::Bool(current)) if current == value
                )
            }
            AnimatorCondition::FloatGreater { parameter, value } => {
                matches!(
                    self.controller.parameters.get(parameter),
                    Some(ParameterValue::Float(current)) if current > value
                )
            }
            AnimatorCondition::Trigger { parameter } => {
                matches!(self.controller.parameters.get(parameter), Some(ParameterValue::Trigger(true)))
            }
        }
    }

    fn consume_triggers(&mut self, index: usize) {
        let names: Vec<String> = self.controller.transitions[index]
            .conditions
            .iter()
            .filter_map(|condition| match condition {
                AnimatorCondition::Trigger { parameter } => Some(parameter.clone()),
                _ => None,
            })
            .collect();
        for name in names {
            self.controller.parameters.insert(name, ParameterValue::Trigger(false));
        }
    }
}
