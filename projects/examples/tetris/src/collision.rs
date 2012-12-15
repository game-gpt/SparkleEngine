//! 碰撞与落定。

use crate::board::Board;
use crate::pieces::{PieceKind, cells};

pub fn in_bounds(board: &Board, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as u32) < board.width && (y as u32) < board.height
}

pub fn fits(board: &Board, kind: PieceKind, rot: u8, ox: i32, oy: i32) -> bool {
    for (cx, cy) in cells(kind, rot) {
        let x = ox + cx;
        let y = oy + cy;
        if y < 0 {
            if x < 0 || x as u32 >= board.width {
                return false;
            }
            continue;
        }
        if !in_bounds(board, x, y) || board.get(x, y) != 0 {
            return false;
        }
    }
    true
}

pub fn lock_piece(board: &mut Board, kind: PieceKind, rot: u8, ox: i32, oy: i32) {
    for (cx, cy) in cells(kind, rot) {
        let x = ox + cx;
        let y = oy + cy;
        if y >= 0 {
            board.set(x, y, kind.0);
        }
    }
}
