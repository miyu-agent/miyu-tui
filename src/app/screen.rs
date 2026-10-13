//! 上一帧画出来的（蓝图 `app.md` 的 `screen` 块）：各块位置、正文的视口、光标、抽屉和面板每一行是第几项、侧边栏能选的行、
//! 鼠标在哪、按着的是哪一块，和画的时候要的主题名、启动时刻、随机数。画的时候填，鼠标、点击照它认。

use std::time::Instant;

use ratatui::layout::Position;

use super::Grab;
use crate::body_view::BodyView;
use crate::side_select::SideSelect;
use crate::ui::Areas;

/// 上一帧画出来的。
pub struct Screen {
    /// 上一帧各块的位置。
    pub areas: Areas,
    /// 正文区：滚到哪、悬在哪、选了哪。
    pub view: BodyView,
    /// 这一帧的光标停在哪、显不显示；画的时候填，画完 `main` 写出去（蓝图「每一帧」）。
    pub caret: crate::caret::Caret,
    /// 抽屉每一行是第几项（点哪一行点中哪一项）；上一帧排出来的。
    pub drawer_rows: Vec<Option<usize>>,
    /// 后台面板每一行是第几条命令（点哪一行点中哪一条）；上一帧排出来的。
    pub panel_rows: Vec<Option<usize>>,
    /// 鼠标悬停在子代理状态行的第几行（第 0 行是上面的空行）。
    pub agents_hover: Option<usize>,
    /// 鼠标最后在哪一格；还没动过是 `None`。
    pub pointer: Option<Position>,
    /// 按着鼠标时是哪一块的。
    pub grab: Option<Grab>,
    /// 侧边栏的选字（`tui.md`「后台命令、子代理和侧边栏」第 7 条）。
    pub side_select: SideSelect,
    /// 当前主题的名字（出厂那一份的）。
    pub theme: String,
    /// 主题的几个配置项（`tui.theme*`，「主题」第 6 条）。
    pub theme_choice: crate::theme::choice::Choice,
    /// 起来时问到的终端底色是不是深色；问不到的是 `None`（「主题」第 3 条）。
    pub terminal_dark: Option<bool>,
    /// 跟随头像的配色正在用的主色（「主题」第 5 条）；没有的是 `None`。
    pub theme_seed: Option<u32>,
    /// 从头像取出来的主色（后台取，一张只取一次）。
    pub seeds: crate::theme::seeds::Seeds,
    /// 引导「主题」那一步选着的配色、明暗，盖在配置项上（「第一次打开的引导」第 23a 条）；不在选的是 `None`。
    pub theme_preview: Option<(String, crate::theme::choice::Mode)>,
    /// 程序启动的时刻：转圈照它算第几帧。
    pub started: Instant,
    /// 压缩那一行的进度条一顿一顿地追，停多久、追几格要的随机数。
    pub rng: crate::rng::Rng,
}

impl Screen {
    /// 刚起来：什么都还没画；`theme` 是起来时的主题名。
    pub fn new(theme: String) -> Self {
        Self {
            areas: Areas::default(),
            view: BodyView::default(),
            caret: crate::caret::Caret::default(),
            drawer_rows: Vec::new(),
            panel_rows: Vec::new(),
            agents_hover: None,
            pointer: None,
            grab: None,
            side_select: SideSelect::default(),
            theme,
            theme_choice: crate::theme::choice::Choice::default(),
            terminal_dark: None,
            theme_seed: None,
            seeds: crate::theme::seeds::Seeds::default(),
            theme_preview: None,
            started: Instant::now(),
            rng: crate::rng::Rng::from_clock(),
        }
    }
}
