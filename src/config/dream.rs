//! 人打的 `/dream` 整理时那一块（蓝图 `tui.md`「整理记忆」）：字在 `text/*.json` 的 `dream`，样子在 `layout.json` 的 `dream`。

use serde::Deserialize;

/// 整理时那一块的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DreamTexts {
    /// 第一行（第 3 条）。
    pub title: String,
    /// 第一行后面的第几批，`{n}`、`{total}`。
    pub batch: String,
    /// 删的种类：过时。
    pub outdated: String,
    /// 重复。
    pub repeat: String,
    /// 没价值。
    pub worthless: String,
    /// 摘要那一行，`{chars}`。
    pub summary: String,
    /// 摘要还在写，接在后面。
    pub writing: String,
    /// 夜空右下角，`{merged}`、`{total}`。
    pub tally: String,
    /// 第一行末尾整理了多久，`{thinking}`（还没写出东西时是 [`DreamTexts::thinking`]，写出了是空的）、`{time}`。
    pub clock: String,
    /// 她还一个字都没写出来（第 3 条）：吉祥物在睡觉，照睡觉说（2026-10-20 项目主人：「在想」不合适）。
    pub thinking: String,
    /// 回执点开列的摘要打头的词。
    pub summary_full: String,
    /// 整理时人说了话、把她叫醒了的提示（第 7 条）。
    pub backgrounded: String,
}

/// 整理时那一块的样子。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DreamLook {
    /// 第一行打头的记号，连同后面的空格。
    pub mark: String,
    /// 滚动区浮过的一条、删、改、摘要打头的记号，连同后面的空格。
    pub float: String,
    /// 删。
    pub retired: String,
    /// 改。
    pub revised: String,
    /// 摘要。
    pub summary: String,
    /// 删的原文和种类之间。
    pub reason_gap: String,
    /// 浮过去一条隔多久，毫秒（第 4 条）。
    pub float_ms: u64,
    /// 吉祥物睡着、醒来的脸（第 5、6 条）。
    pub asleep: String,
    /// 醒来的脸。
    pub awake: String,
    /// 醒来时头上那一个。
    pub wake_mark: String,
    /// 醒来停多久再收成回执，毫秒。
    pub wake_ms: u64,
    /// 左边那一栏几列宽：吉祥物和 z。
    pub left_width: usize,
    /// z 一帧一帧：上一行、脸右边那一截；照 `z_frame_ms` 轮。
    pub z_top: Vec<String>,
    /// 脸右边那一截，和 `z_top` 一样多帧。
    pub z_side: Vec<String>,
    /// z 一帧多久，毫秒。
    pub z_frame_ms: u64,
    /// 亮了的星长什么样，一颗颗轮着用。
    pub glyphs: Vec<String>,
    /// 这一批还没合完的那几颗（微微发亮）。
    pub soon: String,
    /// 没亮的。
    pub unlit: String,
    /// 最多几颗星；一颗至少占几列（放不下按宽度少放）。
    pub stars: usize,
    /// 一颗星至少占几列。
    pub star_cols: usize,
    /// 每几颗里有一颗会闪。
    pub twinkle_every: usize,
    /// 闪一个来回多久，毫秒。
    pub twinkle_ms: u64,
    /// 闪最亮时往白里偏多少，0 到 1。
    pub twinkle_lift: f64,
    /// 这一批的星明暗呼吸一个来回多久，毫秒（一批之内不装进度，第 5 条）。
    pub breathe_ms: u64,
    /// 回执打头的记号，连同后面的空格：暗色（第 6 条）。
    pub receipt: String,
    /// 星星的位置照它定。
    pub seed: u64,
}
