//! 棋盘权威状态。

/// 固定网格棋盘：行主序 `cells[y * width + x]`，`0` 为空，非零为 [`crate::PieceKind`] 取值。
#[derive(Debug, Clone)]
pub struct Board {
    /// 列数（格子）。默认 10。
    pub width: u32,
    /// 行数（格子）。默认 20；`y = 0` 为顶行。
    pub height: u32,
    /// 行主序格子；长度恒为 `width * height`。
    pub cells: Vec<u8>,
}

impl Default for Board {
    fn default() -> Self {
        let width = 10;
        let height = 20;
        Self { width, height, cells: vec![0; (width * height) as usize] }
    }
}

impl Board {
    /// 读取 `(x, y)`；越界返回 `0`（不当作墙壁，碰撞由调用方另判）。
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return 0;
        }
        self.cells[(y as u32 * self.width + x as u32) as usize]
    }

    /// 写入 `(x, y)`；越界静默忽略。
    pub fn set(&mut self, x: i32, y: i32, v: u8) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return;
        }
        self.cells[(y as u32 * self.width + x as u32) as usize] = v;
    }

    /// 自底向上消除满行并下沉上方行；返回本轮消除的行数。
    pub fn clear_lines(&mut self) -> u32 {
        let mut cleared = 0_u32;
        let w = self.width as usize;
        let mut y = self.height as i32 - 1;
        while y >= 0 {
            let full = (0..self.width as i32).all(|x| self.get(x, y) != 0);
            if full {
                cleared += 1;
                for row in (1..=y).rev() {
                    for x in 0..w {
                        let from = ((row - 1) as usize) * w + x;
                        let to = (row as usize) * w + x;
                        self.cells[to] = self.cells[from];
                    }
                }
                for x in 0..w {
                    self.cells[x] = 0;
                }
            }
            else {
                y -= 1;
            }
        }
        cleared
    }
}
