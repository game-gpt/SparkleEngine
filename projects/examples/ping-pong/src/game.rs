//! 游戏壳：注册可反射组件的占位宿主。

use spark_renderer::{DrawList, FrameCtx, GameHost};

use crate::ball::Ball;
use crate::paddle::Paddle;

#[derive(Debug, Default)]
pub struct PingPongApp {
    pub left: Paddle,
    pub right: Paddle,
    pub ball: Ball,
    exit: bool,
}

impl GameHost for PingPongApp {
    fn update(&mut self, _frame: &FrameCtx<'_>) {
        // 输入与物理接入后在此推进
        let _ = (&mut self.left, &mut self.right, &mut self.ball);
    }

    fn draw(&mut self, _draw: &mut DrawList) {}

    fn should_exit(&self) -> bool {
        self.exit
    }
}
