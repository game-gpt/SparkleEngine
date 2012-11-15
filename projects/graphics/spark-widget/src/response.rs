//! 事件与控件响应。

use crate::command::UiCommand;

#[derive(Debug, Default)]
pub struct EventResponse {
    pub handled: bool,
    pub commands: Vec<UiCommand>,
}

impl EventResponse {
    pub fn handled() -> Self {
        Self {
            handled: true,
            commands: Vec::new(),
        }
    }

    pub fn with_command(command: UiCommand) -> Self {
        Self {
            handled: true,
            commands: vec![command],
        }
    }
}
