//! 输入历史列表的框多高、怎么排（蓝图 `tui.md`「输入历史列表」第 1 条「高度」「宽窗口预览放右边」，2026-11-12 项目主人，
//! 发版前 check 第 52 条：原来列表最多 8 条、预览照露着的几条里最长的那条高，一滚动框就一伸一缩）：框的高度打开时定死，
//! 上下选、滚动、搜索筛少了都不变；只照全部历史有几条、窗口多大、两个配置项算。画（`lines`）和鼠标（`app/mouse.rs`）照同一份。

use crate::config::Layout;

/// 框里够这么多列，预览放到右边（同一天项目主人定）。
pub const SIDE_FROM: u16 = 100;

/// 预览在右边时列表占框里的几成（百分比）、至少几列。
const LEFT_SHARE: u16 = 45;
const LEFT_MIN: u16 = 40;

/// `tui.history_rows` 是 0（跟窗口高）时列表那一段限在几条之间。
const AUTO_MIN: usize = 8;
const AUTO_MAX: usize = 20;

/// 上下排时框里除了列表、预览的字以外占的行：框的上下两条边、预览小框的上下两条边、横线、搜索那一行。
const STACKED_CHROME: usize = 6;
/// 预览在右边时框里除了列表那一段以外占的行：框的上下两条边、横线、搜索那一行。
const SIDE_CHROME: usize = 4;

/// 框里 `width` 列时预览放不放右边。
pub fn side(width: u16) -> bool {
    width >= SIDE_FROM
}

/// 预览在右边时列表那一半几列（竖线画在它右边那一格）。
pub fn left_width(width: u16) -> u16 {
    (width * LEFT_SHARE / 100).max(LEFT_MIN).min(width)
}

/// 上下排时预览固定几行字（`tui.history_preview_rows`，至少 1 行）。
pub fn preview_rows(layout: &Layout) -> usize {
    layout.history_preview_rows.max(1)
}

/// 列表那一段几行：`tui.history_rows` 写了数照数，0 照窗口（`screen` 行）算，整个框约占窗口一半、限在 8 到 20 条；
/// 历史（`total` 条）比这少的照历史的条数，至少 1 行（放「没有对得上的」）。照全部历史算，不照对得上的条数：筛少了框不缩。
pub fn list_rows(layout: &Layout, total: usize, width: u16, screen: u16) -> usize {
    let wanted = if layout.history_rows > 0 {
        layout.history_rows
    } else {
        let chrome = if side(width) {
            SIDE_CHROME
        } else {
            preview_rows(layout) + STACKED_CHROME
        };
        (usize::from(screen) / 2)
            .saturating_sub(chrome)
            .clamp(AUTO_MIN, AUTO_MAX)
    };
    wanted.min(total.max(1))
}
