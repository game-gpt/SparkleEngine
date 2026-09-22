//! 对白与选项播放器。
//!
//! # 不变式
//!
//! - 队列按 [`ScriptPlayer::push_line`] / [`ScriptPlayer::push_choices`] 顺序推进。
//! - 进入选项事件后进入「等待选择」：[`ScriptPlayer::advance`] 反复返回同一组选项，直到 [`ScriptPlayer::choose`] 成功。
//! - [`ScriptPlayer::choose`] 在「无等待选项」时失败且不改状态；索引越界时已 `take` 掉选项（等待态清空），错误码 `spark.galgame/script.choice_invalid`。

use spark_types::{ErrorArg, ErrorCode, SparkError};

fn choice_invalid() -> ErrorCode {
    ErrorCode::new("spark.galgame", "script.choice_invalid")
}

use crate::flags::FlagStore;

/// 一行对白（说话人 / 文本 / 可选语音键）。
#[derive(Debug, Clone)]
pub struct DialogueLine {
    /// 说话人显示键；旁白可为 `None`。
    pub speaker: Option<String>,
    /// 对白正文。
    pub text: String,
    /// 可选语音资源键（不透明）。
    pub voice: Option<String>,
}

/// 玩家可选的一项。
#[derive(Debug, Clone)]
pub struct Choice {
    /// 选项显示文案。
    pub label: String,
    /// 选中时写入旗标。
    pub set_flag: Option<(String, f64)>,
}

/// 脚本队列中的一条事件（内部排队用）。
#[derive(Debug, Clone)]
pub enum ScriptEvent {
    /// 一行对白。
    Line(DialogueLine),
    /// 一组互斥选项。
    Choices(Vec<Choice>),
}

/// [`ScriptPlayer::advance`] 对外可见状态。
#[derive(Debug, Clone)]
pub enum ScriptState {
    /// 刚消化的一行对白（已记入历史）。
    Line(DialogueLine),
    /// 正在等待玩家选择。
    Choices(Vec<Choice>),
    /// 队列已耗尽且无等待选项。
    Ended,
}

/// 对白 / 选项队列与回看历史。
#[derive(Debug, Default)]
pub struct ScriptPlayer {
    queue: Vec<ScriptEvent>,
    cursor: usize,
    waiting_choice: Option<Vec<Choice>>,
    /// 已播放过的对白（不含选项本身）。
    pub history: Vec<DialogueLine>,
}

impl ScriptPlayer {
    /// 追加一行对白事件。
    pub fn push_line(&mut self, line: DialogueLine) {
        self.queue.push(ScriptEvent::Line(line));
    }

    /// 追加一组选项事件。
    pub fn push_choices(&mut self, choices: Vec<Choice>) {
        self.queue.push(ScriptEvent::Choices(choices));
    }

    /// 清空队列、游标、等待选项与历史。
    pub fn clear(&mut self) {
        self.queue.clear();
        self.cursor = 0;
        self.waiting_choice = None;
        self.history.clear();
    }

    /// 推进一拍。若正在等待选择则仍返回该选项组；否则取下一事件或 [`ScriptState::Ended`]。
    pub fn advance(&mut self, _flags: &mut FlagStore) -> ScriptState {
        if self.waiting_choice.is_some() {
            return ScriptState::Choices(self.waiting_choice.clone().unwrap());
        }
        if self.cursor >= self.queue.len() {
            return ScriptState::Ended;
        }
        let ev = self.queue[self.cursor].clone();
        self.cursor += 1;
        match ev {
            ScriptEvent::Line(line) => {
                self.history.push(line.clone());
                ScriptState::Line(line)
            }
            ScriptEvent::Choices(c) => {
                self.waiting_choice = Some(c.clone());
                ScriptState::Choices(c)
            }
        }
    }

    /// 在等待选项时按索引选择。
    ///
    /// # 错误
    ///
    /// - 当前无等待选项：`reason=no_choices`
    /// - 索引越界：`reason=index_out_of_bounds`，附带 `index` / `len`
    ///
    /// 失败时不消费选项、不改旗标。
    pub fn choose(&mut self, index: usize, flags: &mut FlagStore) -> Result<(), SparkError> {
        let Some(choices) = self.waiting_choice.take()
        else {
            return Err(SparkError::new(choice_invalid()).arg("reason", ErrorArg::String("no_choices".into())));
        };
        let Some(c) = choices.get(index)
        else {
            return Err(SparkError::new(choice_invalid())
                .arg("reason", ErrorArg::String("index_out_of_bounds".into()))
                .arg("index", ErrorArg::Unsigned(index as u64))
                .arg("len", ErrorArg::Unsigned(choices.len() as u64)));
        };
        if let Some((k, v)) = &c.set_flag {
            flags.set(k, *v);
        }
        Ok(())
    }
}
