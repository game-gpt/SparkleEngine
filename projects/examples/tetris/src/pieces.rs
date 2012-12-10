//! 方块形状表（Rust）。

#[derive(Debug, Clone, Copy)]
pub struct PieceKind(pub u8);

pub fn cell_count(kind: PieceKind) -> usize {
    match kind.0 {
        0..=6 => 4,
        _ => 0,
    }
}
