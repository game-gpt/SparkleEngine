//! 七种方块与旋转。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceKind(pub u8);

/// 相对轴心的四个格子（未旋转）。
pub fn cells(kind: PieceKind, rot: u8) -> [(i32, i32); 4] {
    let base = match kind.0 {
        1 => [(0, 1), (1, 1), (2, 1), (3, 1)], // I
        2 => [(1, 0), (2, 0), (1, 1), (2, 1)], // O
        3 => [(1, 0), (0, 1), (1, 1), (2, 1)], // T
        4 => [(1, 0), (2, 0), (0, 1), (1, 1)], // S
        5 => [(0, 0), (1, 0), (1, 1), (2, 1)], // Z
        6 => [(0, 0), (0, 1), (1, 1), (2, 1)], // J
        7 => [(2, 0), (0, 1), (1, 1), (2, 1)], // L
        _ => [(0, 0), (0, 0), (0, 0), (0, 0)],
    };
    if kind.0 == 2 {
        return base;
    }
    let mut out = base;
    let turns = (rot % 4) as usize;
    for _ in 0..turns {
        for p in &mut out {
            // 绕 (1.5, 1.5) 附近顺时针：对 I 用 (1,1) 近似
            let (x, y) = *p;
            *p = (1 - (y - 1), x);
        }
    }
    out
}

pub fn rotate_cw(rot: u8) -> u8 {
    (rot + 1) % 4
}

pub fn random_kind(rng: &mut u64) -> PieceKind {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
    PieceKind(((*rng >> 33) % 7) as u8 + 1)
}
