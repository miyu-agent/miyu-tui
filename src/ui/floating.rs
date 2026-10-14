//! 悬浮窗（蓝图「配置页」第 14、15、20 条）：浅一档的底、不画边框；标题、一行一项，选中的那一行左边强调色竖条、底色深一档；
//! 报错、按钮、按键提示。配置页的几种窗都照它画；引导「主题」那一步的选择窗也是它（2026-10-14 项目主人：和配置页选值的
//! 列表同一个组件、同一套键）。点到哪一行交回什么，各页自己定（`H`）。

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::caret::Caret;
use crate::settings::pages::choose::Choose;
use crate::theme;

/// 窗里的一行行，画之前先排好。
pub struct Lines<H> {
    pub title: Option<(String, String)>,
    pub body: Vec<Body<H>>,
    pub error: Option<String>,
    pub buttons: Vec<(String, bool)>,
    pub button: Option<usize>,
    pub hints: Vec<[String; 2]>,
    /// 正在改的那一行：第几行、光标在第几列。
    pub caret: Option<(usize, u16)>,
}

impl<H> Default for Lines<H> {
    fn default() -> Self {
        Self {
            title: None,
            body: Vec::new(),
            error: None,
            buttons: Vec::new(),
            button: None,
            hints: Vec::new(),
            caret: None,
        }
    }
}

/// 一行：字、选中没有、点到它是什么。
pub struct Body<H> {
    pub line: Line<'static>,
    pub focused: bool,
    pub hit: Option<H>,
}

impl<H> Lines<H> {
    pub fn title(&mut self, title: String, sub: String) {
        self.title = Some((title, sub));
    }

    pub fn plain(&mut self, line: Line<'static>) {
        self.body.push(Body {
            line,
            focused: false,
            hit: None,
        });
    }

    /// 窗有多高：一行一项，加上标题、按钮、按键提示和中间的空行，报错折几行就多几行；最高 `max`。
    pub fn height(&self, width: u16, max: u16) -> u16 {
        let errors = self.error.as_deref().map_or(0, |e| {
            error_rows(e, usize::from(width.saturating_sub(4))).len()
        });
        (self.body.len() as u16 + 8 + errors as u16).min(max)
    }
}

/// 选择窗的一行行：一个选项一行，选不了的暗着、后面黄字写原因，有说明的暗色接在后面。点到第几行交回 `row(i)`。
pub fn choose<H>(
    lines: &mut Lines<H>,
    choose: &Choose,
    hints: Vec<[String; 2]>,
    width: u16,
    row: impl Fn(usize) -> H,
) {
    lines.title(choose.title.clone(), String::new());
    let room = usize::from(width.saturating_sub(6));
    for (i, c) in choose.choices.iter().enumerate() {
        let style = if c.off.is_some() {
            theme::dim()
        } else {
            Style::new()
        };
        let mut spans = vec![Span::raw("  "), Span::styled(fit(&c.label, room), style)];
        let used = c.label.width() + 2;
        if let Some(off) = &c.off {
            spans.push(Span::styled(
                fit(&format!("  {off}"), room.saturating_sub(used)),
                theme::warn(),
            ));
        } else if let Some(note) = &c.note {
            spans.push(Span::styled(
                fit(&format!("  {note}"), room.saturating_sub(used)),
                theme::dim(),
            ));
        }
        lines.body.push(Body {
            line: Line::from(spans),
            focused: i == choose.sel,
            hit: Some(row(i)),
        });
    }
    lines.hints = hints;
}

/// 在 `area` 正中画一个 `width` 宽的窗：点得到的行、按钮记进 `hits`，按钮交回 `button(i)`。
pub fn centered<H: Copy>(
    frame: &mut Frame,
    area: Rect,
    (width, max_height): (u16, u16),
    lines: &mut Lines<H>,
    (hits, button): (&mut Vec<(Rect, H)>, impl Fn(usize) -> H),
    caret: &mut Caret,
) {
    let height = lines.height(width, max_height);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    paint(frame, rect, lines, (hits, button), caret);
}

/// 铺底色、写标题、滚着写一行行、出错的一句、按钮、按键提示。
fn paint<H: Copy>(
    frame: &mut Frame,
    rect: Rect,
    lines: &mut Lines<H>,
    (hits, button): (&mut Vec<(Rect, H)>, impl Fn(usize) -> H),
    caret: &mut Caret,
) {
    // 先把格子整个清掉（字色、修饰都不留，不然窗里的字带着后面那页的暗色），再铺底色。
    let buf = frame.buffer_mut();
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                cell.reset();
            }
        }
    }
    buf.set_style(rect, theme::panel());
    let inner = Rect::new(
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        rect.height.saturating_sub(2),
    );
    if let Some((title, sub)) = &lines.title {
        buf.set_string(
            inner.x,
            inner.y,
            fit(title, inner.width as usize),
            theme::accent().add_modifier(Modifier::BOLD),
        );
        let w = sub.width() as u16;
        if !sub.is_empty() && w + title.width() as u16 + 2 < inner.width {
            buf.set_string(inner.right() - w, inner.y, sub, theme::dim());
        }
    }
    let errors = lines
        .error
        .as_deref()
        .map(|e| error_rows(e, usize::from(inner.width)))
        .unwrap_or_default();
    let room = inner.height.saturating_sub(6 + errors.len() as u16) as usize;
    let focus = lines.body.iter().position(|b| b.focused).unwrap_or(0);
    let top = focus
        .saturating_sub(room.saturating_sub(1))
        .min(lines.body.len().saturating_sub(room));
    let top = if lines.caret.is_some_and(|(row, _)| row == 0) {
        0
    } else {
        top
    };
    for (offset, body) in lines.body.iter().skip(top).take(room).enumerate() {
        let y = inner.y + 2 + offset as u16;
        let row = Rect::new(inner.x, y, inner.width, 1);
        if body.focused {
            frame.buffer_mut().set_style(row, theme::row_focus());
            bar(frame, row);
        }
        frame.buffer_mut().set_line(row.x, y, &body.line, row.width);
        if let Some(hit) = body.hit {
            hits.push((row, hit));
        }
        if let Some((at, x)) = lines.caret
            && at == top + offset
        {
            caret.put(
                Position::new((row.x + x).min(row.right().saturating_sub(1)), y),
                true,
            );
        }
    }
    let mut y = inner.bottom().saturating_sub(3);
    // 报错、按钮、按键提示：落在窗里的才画。终端只有四五行高时它们算到窗外，原来 `y - 1` 下溢崩溃（review A7）。
    let inside = |y: u16| y >= inner.y && y < inner.bottom();
    // 报错折行写全（最多三行），写在按钮上面（2026-10-13 验收 T-181：原来一行截掉，「最长 64 个字符」看不到）。
    for (i, text) in errors.iter().rev().enumerate() {
        if let Some(above) = y.checked_sub(1 + i as u16).filter(|&r| inside(r)) {
            frame
                .buffer_mut()
                .set_string(inner.x, above, text, theme::error());
        }
    }
    let mut x = inner.x;
    let buttons = if inside(y) {
        lines.buttons.as_slice()
    } else {
        &[]
    };
    for (i, (label, danger)) in buttons.iter().enumerate() {
        let text = format!("[ {label} ]");
        let on = lines.button == Some(i);
        let style = match (on, danger) {
            (true, true) => theme::error().add_modifier(Modifier::REVERSED),
            (true, false) => theme::picked_bar(),
            _ => Style::new(),
        };
        let w = text.width() as u16;
        frame.buffer_mut().set_string(x, y, &text, style);
        hits.push((Rect::new(x, y, w, 1), button(i)));
        x += w + 2;
    }
    // 按钮和按键提示之间空一行（2026-10-07 项目主人）。
    y += 2;
    if inside(y) {
        frame
            .buffer_mut()
            .set_line(inner.x, y, &hint_line(&lines.hints), inner.width);
    }
}

/// 报错折成几行：最多三行，还放不下的第三行截掉加「…」。
fn error_rows(error: &str, width: usize) -> Vec<String> {
    let mut rows = crate::cells::wrap(error, width.max(1));
    if rows.len() > 3 {
        let rest: String = rows.split_off(2).concat();
        rows.push(fit(&rest, width));
    }
    rows
}

/// 一行按键提示：整行暗下去，键暗色、说明更暗，每格隔三列（「配置页」第 42 条，2026-10-09 项目主人：「底下这种按键提示
/// 我觉得才应该是暗色」；原来键是黄的，比内容还亮）。
pub fn hint_line(hints: &[[String; 2]]) -> Line<'_> {
    let mut spans = Vec::new();
    for (i, [key, what]) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(key.as_str(), theme::dim()));
        spans.push(Span::styled(format!(" {what}"), theme::faint()));
    }
    Line::from(spans)
}

/// 选中的那一行左边的强调色竖条。
pub fn bar(frame: &mut Frame, row: Rect) {
    for y in row.y..row.bottom() {
        frame
            .buffer_mut()
            .set_string(row.x, y, "┃", theme::accent());
    }
}

/// 字太长裁掉、末尾写「…」。
pub fn fit(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut out = String::new();
    for c in text.chars() {
        if out.width() + unicode_width::UnicodeWidthChar::width(c).unwrap_or(0) + 1 > width {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}
