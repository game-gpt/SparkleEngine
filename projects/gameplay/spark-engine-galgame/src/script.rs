//! 对白与选项播放器。

use spark_core::{ErrorArg, ErrorCode, SparkError};

fn choice_invalid() -> ErrorCode {
    ErrorCode::new("spark.galgame", "script.choice_invalid")
}

use crate::flags::FlagStore;

#[derive(Debug, Clone)]
pub struct DialogueLine {
    pub speaker: Option<String>,
    pub text: String,
    pub voice: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Choice {
    pub label: String,
    /// 选中时写入旗标。
    pub set_flag: Option<(String, f64)>,
}

#[derive(Debug, Clone)]
pub enum ScriptEvent {
    Line(DialogueLine),
    Choices(Vec<Choice>),
}

#[derive(Debug, Clone)]
pub enum ScriptState {
    Line(DialogueLine),
    Choices(Vec<Choice>),
    Ended,
}

#[derive(Debug, Default)]
pub struct ScriptPlayer {
    queue: Vec<ScriptEvent>,
    cursor: usize,
    waiting_choice: Option<Vec<Choice>>,
    pub history: Vec<DialogueLine>,
}

impl ScriptPlayer {
    pub fn push_line(&mut self, line: DialogueLine) {
        self.queue.push(ScriptEvent::Line(line));
    }

    pub fn push_choices(&mut self, choices: Vec<Choice>) {
        self.queue.push(ScriptEvent::Choices(choices));
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.cursor = 0;
        self.waiting_choice = None;
        self.history.clear();
    }

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
