//! 碰撞检测（Rust）。

use crate::board::Board;

pub fn in_bounds(board: &Board, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as u32) < board.width && (y as u32) < board.height
}
