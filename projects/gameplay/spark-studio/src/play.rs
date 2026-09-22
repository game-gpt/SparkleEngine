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
        if let Some(session) = resolve_by_target(project) {
            return Ok(session);
        }
        // 未声明 runTarget 时按项目名 / kind 兜底，避免 Play 按钮「点了没反应」。
        if let Some(session) = resolve_by_name(&project.name) {
            return Ok(session);
        }
        match project.kind {
            ProjectKind::Valkyrie => Ok(Self::Snake(SnakeApp::new())),
            ProjectKind::Hybrid => Ok(Self::Tetris(TetrisApp::new())),
            ProjectKind::Rust => Ok(Self::PingPong(PingPongGame::new())),
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

    pub fn label(&self) -> &'static str {
        match self {
            Self::PingPong(_) => "ping-pong",
            Self::Tetris(_) => "tetris",
            Self::Snake(_) => "snake",
        }
    }
}

fn resolve_by_target(project: &ProjectInfo) -> Option<PlaySession> {
    let raw = project.run_target.as_deref()?;
    resolve_token(raw)
}

fn resolve_by_name(name: &str) -> Option<PlaySession> {
    resolve_token(name)
}

fn resolve_token(raw: &str) -> Option<PlaySession> {
    let key = raw.trim().to_ascii_lowercase().replace('_', "-");
    if key.contains("ping") || key == "ping-pong" || key == "pingpong" {
        return Some(PlaySession::PingPong(PingPongGame::new()));
    }
    if key.contains("tetris") {
        return Some(PlaySession::Tetris(TetrisApp::new()));
    }
    if key.contains("snake") {
        return Some(PlaySession::Snake(SnakeApp::new()));
    }
    None
}
