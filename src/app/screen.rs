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
    /// 这一帧当急的画，不等下一拍（会话补完换上来；`main.rs` 取走）。
    pub draw_now: bool,
    /// 把终端整屏擦一遍：删掉贴上去的图、清屏、下一帧整屏重画（`/images off`；`main.rs` 取走）。
    pub wipe: bool,
    /// 正文区：滚到哪、悬在哪、选了哪。
    pub view: BodyView,
    /// 这一帧的光标停在哪、显不显示；画的时候填，画完 `main` 写出去（蓝图「每一帧」）。
    pub caret: crate::caret::Caret,
    /// 抽屉每一行是第几项（点哪一行点中哪一项）；上一帧排出来的。
    pub drawer_rows: Vec<Option<usize>>,
    /// 后台面板每一行是第几条命令（点哪一行点中哪一条）；上一帧排出来的。
    pub panel_rows: Vec<Option<usize>>,
    /// 侧边栏「最近会话」每一行是哪个会话，照 `areas.recent` 从上往下；上一帧排出来的。
    pub recent: Vec<String>,
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
    /// 起来时问到的终端底色是不是深色；问不到的是 `None`（「主题」第 3 条）。拿回焦点再问到的盖上。
    pub terminal_dark: Option<bool>,
    /// 拿回了焦点、主题跟着终端的颜色：这一拍过后再问一次终端的颜色（`main.rs` 取走，「主题」第 11 条）。
    pub ask_colors: bool,
    /// 跟随头像、跟随壁纸的配色正在用的主色（「主题」第 5 条）；没有的是 `None`。
    pub theme_seed: Option<u32>,
    /// 能当主色的几个候选，好的在前（头像的核心给，壁纸的门面算；外观页「主色」照它列）。
    pub theme_seeds: Vec<u32>,
    /// 跟随壁纸自选的主色（`tui.theme_seed`）：在候选里才用，不然照打分最高的那个。
    pub theme_pick: Option<u32>,
    /// 从头像取出来的主色（后台取，一张只取一次）。
    pub seeds: crate::theme::seeds::Seeds,
    /// 跟随壁纸：后台找桌面壁纸、matugen 那一份（`theme/wallpaper.rs`）。
    pub wallpaper: crate::theme::wallpaper::Wallpaper,
    /// 起来以后找过一次壁纸了。
    pub wallpaper_seen: bool,
    /// 上一拍要不要壁纸（开着配色列表、选着跟随壁纸）：从不要变成要的时候再找一次。
    pub wallpaper_wanted: bool,
    /// 引导「主题」那一步选着的配色、明暗，盖在配置项上（「第一次打开的引导」第 23a 条）；不在选的是 `None`。
    pub theme_preview: Option<crate::oobe::theme::Preview>,
    /// 引导挑过的主色：关引导以后、人格列表重读回来以前照它（`theme/seed_hold.rs`）。
    pub seed_hold: crate::theme::seed_hold::SeedHold,
    /// 程序启动的时刻：转圈照它算第几帧。
    pub started: Instant,
    /// 压缩那一行的进度条一顿一顿地追，停多久、追几格要的随机数。
    pub rng: crate::rng::Rng,
    /// 这一帧界面块能点的地方（蓝图「界面块」第 7 条）：画的时候记，鼠标照它分。
    pub widget_spots: Vec<crate::ui::widget::Spot>,
    /// 指针停在底栏哪个界面块的那一格上（包、编号）：换了一格才提示一次是谁放的。
    pub status_hover: Option<(String, String)>,
    /// 上一帧扩展的整页画成什么样：键盘照它选、滚（`app/widget_page.rs`）。
    pub widget_page: crate::ui::widget::PageFrame,
    /// 拖滚动条时按像素报鼠标（`pixel_mouse.rs`）。
    pub pixel: crate::pixel_mouse::PixelMouse,
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
            recent: Vec::new(),
            agents_hover: None,
            pointer: None,
            grab: None,
            side_select: SideSelect::default(),
            theme,
            theme_choice: crate::theme::choice::Choice::default(),
            terminal_dark: None,
            ask_colors: false,
            theme_seed: None,
            theme_seeds: Vec::new(),
            theme_pick: None,
            seeds: crate::theme::seeds::Seeds::default(),
            wallpaper: crate::theme::wallpaper::Wallpaper::default(),
            wallpaper_seen: false,
            wallpaper_wanted: false,
            theme_preview: None,
            seed_hold: Default::default(),
            draw_now: false,
            wipe: false,
            started: Instant::now(),
            rng: crate::rng::Rng::from_clock(),
            widget_spots: Vec::new(),
            status_hover: None,
            widget_page: Default::default(),
            pixel: Default::default(),
        }
    }
}
