//! 棋盘权威状态（Rust）。

#[derive(Debug, Clone)]
pub struct Board {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<u8>,
}

impl Default for Board {
    fn default() -> Self {
        let width = 10;
        let height = 20;
        Self {
            width,
            height,
            cells: vec![0; (width * height) as usize],
        }
    }
}
