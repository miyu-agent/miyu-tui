//! 一帧里各块排在哪（从 `mod.rs` 拆出来，2026-10-12：那个文件过了 500 行）：对话那一屏的输入框、列表、正文、运行状态行、
//! 子代理状态行、待办各占哪几行，输入框和字有多宽。首页的照 `home.rs`。

use ratatui::layout::Rect;

use crate::config::Layout;

/// 一帧里各块的位置。
#[derive(Debug, Clone, Copy, Default)]
pub struct Areas {
    /// 正文。左右和输入框对齐。
    pub body: Rect,
    /// 输入框上面的三样（斜杠命令列表、历史列表、后台面板）连框：贴在输入框上面，没开时高是 0。左右和输入框的边对齐。
    pub menu: Rect,
    /// 那三样框里放字的那一块：左右和输入框里的字对齐（`super::panel::inside`）。
    pub menu_text: Rect,
    /// 输入框，含边框。
    pub frame: Rect,
    /// 输入框里放文字的地方。
    pub text: Rect,
    /// 输入框下面一行：权限级别、模型、用量。左右和框里的字对齐。
    pub footer: Rect,
    /// 运行状态行：在回答时写「深度求索」和用时，提示靠右；没在回答时只有提示，靠左（`tui.md`「运行状态行和排队的消息」）。
    pub pulse: Rect,
    /// 排队的消息，一条一行，列在运行状态行下面；没有的高是 0。
    pub queued: Rect,
    /// 框上面紧挨着的那一行（和框里的字一样宽）：长期目标的进度右对齐写在这里（`goal_tag.rs`）；放不下的高是 0。
    pub goal: Rect,
    /// 首页的吉祥物；不在首页、放不下时高是 0（`tui.md`「空会话的首页」）。
    pub mascot: Rect,
    /// 框下面那一行中间的后台按钮；没有时宽是 0（`tui.md`「后台命令、子代理和侧边栏」第 1 条）。
    pub button: Rect,
    /// 输入框上面常驻的待办；没有时高是 0（第 4、4a 条）。
    pub todo: Rect,
    /// 侧边栏「最近会话」一行一个会话的那几行（不含标题）；没有时高是 0（第 7 条）。哪一行是哪个会话在 `Screen::recent`。
    pub recent: Rect,
    /// 首页框下面那一行再下面（空一行）的工作目录；不在首页是空的（「空会话的首页」第 2 条）。
    pub cwd: Rect,
    /// 子代理状态行（连上面的空行）；没有时高是 0（第 5 条）。
    pub agents: Rect,
    /// 右边的侧边栏（不含竖线）；窗口窄时宽是 0（第 7 条）。
    pub sidebar: Rect,
    /// 回到底部的按钮；在底部、放不下时是空的（`bottom_button.rs`）。
    pub bottom: Rect,
    /// 侧边栏里的短编号那一行：点它复制完整的会话编号（第 7 条）。
    pub session_id: Rect,
    /// 首页工作目录下面放界面块的那一块（蓝图「界面块」第 2 条）；不在首页、没有块的是空的。
    pub widgets: Rect,
}

/// 算出各块的位置。输入框的高度跟着文字的行数走，所以要先定宽度再定高度：`rows` 照框里的字多宽交回占几行
/// （抽屉开着时是抽屉的行数）。`menu_rows` 是列表要露出几行。
/// `running` 在回答，`queued` 排着几条消息，`agent_rows` 是子代理状态行占几行（连上面的空行），
/// `todo_rows` 是窄屏常驻的待办占几行（不连下面的空行）。
#[allow(clippy::too_many_arguments)]
pub fn areas(
    area: Rect,
    rows: &dyn Fn(u16) -> u16,
    layout: &Layout,
    menu_rows: u16,
    running: bool,
    queued: u16,
    agent_rows: u16,
    todo_rows: u16,
) -> Areas {
    let width = box_width(area.width, layout);
    let x = area.x + (area.width - width) / 2;
    let m = super::margins::margins(layout, area.width);
    let pad_left = m.pad_left;
    let text_width = width.saturating_sub(2 + pad_left + m.pad_right).max(1);
    let rows = rows(text_width);
    // 上下两条边和文字；一行都没有的（抽屉让开了）连边也不要。
    let height = if rows == 0 { 0 } else { rows + 2 };
    let footer_y = area.bottom().saturating_sub(1 + agent_rows);
    let frame = Rect::new(x, footer_y.saturating_sub(height), width, height).intersection(area);
    let inner_x = frame.x + 1 + pad_left;
    let text = Rect::new(inner_x, frame.y + 1, text_width, rows).intersection(frame);
    let footer = Rect::new(inner_x, footer_y, text_width, 1).intersection(area);
    // 列表是覆盖层：贴着输入框、盖在正文底部上，正文和下面那几样照没开框时的位置（`tui.md`「斜杠命令列表」第 1 条，
    // 2026-10-02 项目主人定：原来把正文往上推，高度一变正文就上下跳）。只用输入框上面剩下的行（「窗口小的时候」
    // 第 1 条）。
    let menu = above(frame.y, area.y, menu_rows);
    let menu = Rect::new(frame.x, menu.0, frame.width, menu.1);
    // 正文和下面那一块之间：在回答时是 空一行 · 运行状态行 · 排队的消息 · 空一行，再接输入框；没在回答时只空一行
    // （`tui.md`「运行状态行和排队的消息」）。
    // 窄屏的待办常驻在列表（没开时是输入框）上面，下面空一行（`tui.md`「后台命令、子代理和侧边栏」第 4 条）。
    let (todo_y, todo_rows) = above(frame.y.saturating_sub(1), area.y, todo_rows);
    let todo_y = if todo_rows > 0 { todo_y } else { frame.y };
    let block = if running { 3 + queued } else { 1 };
    let block_y = todo_y.saturating_sub(block).max(area.y);
    // 运行状态行那一块放不下的不画，不盖到下面去。
    let free = Rect::new(inner_x, area.y, text_width, todo_y.saturating_sub(area.y));
    let pulse_y = if running { block_y + 1 } else { block_y };
    let pulse = Rect::new(inner_x, pulse_y, text_width, 1).intersection(free);
    let queued_rows = if running { queued } else { 0 };
    let queued = Rect::new(inner_x, pulse_y + 1, text_width, queued_rows).intersection(free);
    // 正文和输入框一样宽，上面空出 top_gap 行。
    let top = (area.y + layout.top_gap).min(block_y);
    let body = Rect::new(frame.x, top, frame.width, block_y.saturating_sub(top));
    Areas {
        body,
        menu,
        menu_text: list_text(menu, inner_x, text_width, layout),
        frame,
        text,
        footer,
        pulse,
        queued,
        goal: Rect::new(inner_x, frame.y.saturating_sub(1), text_width, 1).intersection(Rect::new(
            area.x,
            area.y,
            area.width,
            frame.y.saturating_sub(area.y),
        )),
        agents: agents_area(inner_x, text_width, footer_y + 1, agent_rows, layout)
            .intersection(area),
        todo: Rect::new(inner_x, todo_y, text_width, todo_rows).intersection(area),
        ..Areas::default()
    }
}

/// 输入框上面三样框里放字的那一块：选中的 `❯` 和输入框的提示符同一列，名字和框里打的字同一列
/// （`tui.md`「斜杠命令列表」第 3 条）。`text_x`、`text_width` 是输入框里放字的那一块。
pub(crate) fn list_text(outer: Rect, text_x: u16, text_width: u16, layout: &Layout) -> Rect {
    let x = text_x
        .saturating_sub(prompt_width(layout))
        .max(outer.x.saturating_add(1));
    super::panel::inside(outer, x, text_width + (text_x - x))
}

/// 子代理状态行：每行前面那两格落在提示符那一列，`●` `○` 和框下面那一行的 `▣` 同一列（「后台命令、子代理和
/// 侧边栏」第 6 条）。`text_x`、`text_width` 是输入框里放字的那一块，`y`、`rows` 是从第几行起、几行。
pub(crate) fn agents_area(
    text_x: u16,
    text_width: u16,
    y: u16,
    rows: u16,
    layout: &Layout,
) -> Rect {
    let lead = prompt_width(layout).min(text_x);
    Rect::new(text_x - lead, y, text_width + lead, rows)
}

/// 输入框的提示符（`layout.json` 的 `prompt`）占几列。
fn prompt_width(layout: &Layout) -> u16 {
    u16::try_from(unicode_width::UnicodeWidthStr::width(
        layout.prompt.as_str(),
    ))
    .unwrap_or(0)
}

/// 贴着 `bottom` 往上要 `rows` 行，顶多到 `top`：交回从第几行起、给了几行。
pub(crate) fn above(bottom: u16, top: u16, rows: u16) -> (u16, u16) {
    let rows = rows.min(bottom.saturating_sub(top));
    (bottom - rows, rows)
}

/// 输入框（连边框）多宽：终端宽的几成，两边留白；太窄时、紧凑版面铺满。
pub(crate) fn box_width(area_width: u16, layout: &Layout) -> u16 {
    let m = super::margins::margins(layout, area_width);
    if m.compact {
        return area_width;
    }
    let room = area_width.saturating_sub(m.side_gap * 2);
    let wanted = u16::try_from(u32::from(area_width) * u32::from(layout.width_percent) / 100)
        .unwrap_or(area_width);
    let width = wanted.min(room);
    if width < layout.narrow_below {
        area_width
    } else {
        width
    }
}

/// 框里的字（也是命令列表、历史列表、正文内容）多宽。
pub(super) fn text_width(area_width: u16, layout: &Layout) -> u16 {
    let m = super::margins::margins(layout, area_width);
    box_width(area_width, layout)
        .saturating_sub(2 + m.pad_left + m.pad_right)
        .max(1)
}
