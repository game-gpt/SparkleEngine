//! 编辑器 UI 命令 ID 与 Hierarchy 选中编码。

/// 菜单命令：保存。
pub const CMD_FILE_SAVE: u64 = 100;
/// 菜单命令：撤销。
pub const CMD_EDIT_UNDO: u64 = 110;
/// 工具栏：Play。
pub const CMD_PLAY: u64 = 200;
/// 工具栏：Pause。
pub const CMD_PAUSE: u64 = 201;
/// 工具栏：单步。
pub const CMD_STEP: u64 = 202;
/// 工具栏：Stop（回 Edit）。
pub const CMD_STOP: u64 = 203;
/// 工具：Hand。
pub const CMD_TOOL_HAND: u64 = 300;
/// 工具：Move。
pub const CMD_TOOL_MOVE: u64 = 301;
/// 工具：Rotate。
pub const CMD_TOOL_ROTATE: u64 = 302;
/// 工具：Scale。
pub const CMD_TOOL_SCALE: u64 = 303;
/// 中央标签：Scene。
pub const CMD_TAB_SCENE: u64 = 400;
/// 中央标签：Game。
pub const CMD_TAB_GAME: u64 = 401;
/// 中央标签：Script。
pub const CMD_TAB_SCRIPT: u64 = 402;
/// 底部标签：Project。
pub const CMD_BOTTOM_PROJECT: u64 = 410;
/// 底部标签：Console。
pub const CMD_BOTTOM_CONSOLE: u64 = 411;
/// 底部标签：Problems。
pub const CMD_BOTTOM_PROBLEMS: u64 = 412;
/// 控制台：清空。
pub const CMD_CONSOLE_CLEAR: u64 = 413;
/// 窗口：控件图鉴。
pub const CMD_WINDOW_GALLERY: u64 = 500;
/// 布局预设：默认。
pub const CMD_LAYOUT_DEFAULT: u64 = 600;
/// 布局预设：脚本。
pub const CMD_LAYOUT_SCRIPT: u64 = 601;
/// 布局预设：调试。
pub const CMD_LAYOUT_DEBUG: u64 = 602;
/// 项目资源选中命令基址：实际命令 = 基址 + 列表索引。
pub const CMD_ASSET_BASE: u64 = 700;
/// Hierarchy 选中命令基址：实际命令 = 基址 + 实体 ID。
pub const CMD_SELECT_BASE: u64 = 1000;

/// 编码「选中实体」命令 ID。
pub fn select_cmd(entity_id: u64) -> u64 {
    CMD_SELECT_BASE + entity_id
}

/// 编码「选中资源行」命令 ID。
pub fn asset_cmd(index: u64) -> u64 {
    CMD_ASSET_BASE + index
}

/// 解析资源选中命令；非资源区间返回 `None`。
pub fn parse_asset_cmd(cmd: u64) -> Option<u64> {
    if cmd >= CMD_ASSET_BASE && cmd < CMD_SELECT_BASE { Some(cmd - CMD_ASSET_BASE) } else { None }
}

/// 解析选中命令；非选中区间返回 `None`。
pub fn parse_select_cmd(cmd: u64) -> Option<u64> {
    if cmd >= CMD_SELECT_BASE { Some(cmd - CMD_SELECT_BASE) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_cmd_roundtrip() {
        assert_eq!(parse_asset_cmd(asset_cmd(3)), Some(3));
        assert_eq!(parse_asset_cmd(CMD_SELECT_BASE), None);
        assert_eq!(parse_asset_cmd(CMD_LAYOUT_DEBUG), None);
    }
}
