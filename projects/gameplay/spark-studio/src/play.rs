//! Studio Play：进程内嵌入示例 `GameHost`（Esc / Stop 回编辑器）。

use ping_pong::PingPongGame;
use snake::SnakeApp;
use spark_renderer::{DrawList, FrameCtx, GameHost};
use tetris::TetrisApp;

use crate::project::{ProjectInfo, ProjectKind};

pub enum PlaySession {
    PingPong(PingPongGame),
    Tetris(TetrisApp),
    Snake(SnakeApp),
}

impl PlaySession {
    pub fn start(project: &ProjectInfo) -> Result<Self, String> {
        let target = project
            .run_target
            .as_deref()
            .unwrap_or(project.name.as_str());
        match target {
            "ping-pong" => Ok(Self::PingPong(PingPongGame::new())),
            "tetris" => Ok(Self::Tetris(TetrisApp::new())),
            "snake" => Ok(Self::Snake(SnakeApp::new())),
            other => match project.kind {
                ProjectKind::Valkyrie if other == project.name => Ok(Self::Snake(SnakeApp::new())),
                _ => Err(format!(
                    "尚不支持 Play 目标 `{other}`（已知：ping-pong / tetris / snake）"
                )),
            },
        }
    }

    /// 返回 `true` 表示会话应结束（回编辑器）。
    pub fn update(&mut self, frame: &FrameCtx<'_>) -> bool {
        match self {
            Self::PingPong(g) => {
                g.update(frame);
                g.should_exit()
            }
            Self::Tetris(g) => {
                g.update(frame);
                g.should_exit()
            }
            Self::Snake(g) => {
                g.update(frame);
                g.should_exit()
            }
        }
    }

    pub fn draw(&mut self, draw: &mut DrawList) {
        match self {
            Self::PingPong(g) => g.draw(draw),
            Self::Tetris(g) => g.draw(draw),
            Self::Snake(g) => g.draw(draw),
        }
    }
}
