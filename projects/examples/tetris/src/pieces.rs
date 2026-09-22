//! 七种方块与旋转。

/// 方块种类：格子写入棋盘时用同一数值。
///
/// `1..=7` 依次为 I / O / T / S / Z / J / L；`0` 表示空格，其它值绘制为灰色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceKind(
    /// 种类编码：`1..=7` 为七种方块，与棋盘格存储值一致。
    pub u8,
);

/// 相对轴心的四个格子（未旋转基形，再按 `rot % 4` 顺时针转）。
///
/// 坐标单位为格子；O 块不旋转。I 的旋转枢轴近似为 (1,1)。
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

/// 顺时针下一档旋转索引，结果落在 `0..4`。
pub fn rotate_cw(rot: u8) -> u8 {
    (rot + 1) % 4
}

/// 用 LCG 从 `rng` 抽一种 `1..=7` 方块，并就地推进种子。
pub fn random_kind(rng: &mut u64) -> PieceKind {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
    PieceKind(((*rng >> 33) % 7) as u8 + 1)
}
