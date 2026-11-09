//! 输入框下面那一行：左边权限级别和模型，右边临时的状态和用量。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use unicode_width::UnicodeWidthStr;

use crate::app::{Screen, Seen};
use crate::config::slots::{FOOTER_LEFT, FOOTER_RIGHT, Slot, arrange};
use crate::focus::{Button, Focus};
use crate::theme;
use crate::transcript::Link;
use crate::ui::usage_cell::{self, Hit};
use crate::ui::widget::{Kind, Spot};

/// 左右两段之间至少空几列。
const GAP: usize = 2;

/// 画那一行。左边 `⏵⏵ 工作区 · deepseek-flash deepseek`；右边是每秒 token · 上下文 · `Σ7.0k(C49%)`，
/// 还不知道的那一格不写。有后台命令在跑时，中间的空当正中是后台按钮（`tui.md`「后台命令、子代理和侧边栏」第 1 条）；
/// 返回按钮的位置。
///
/// 一行放不下时，右边的格子照顺序往下丢，上下文留到最后（`13-终端界面.md` 第八节）：左右两段不许叠在一起，
/// 叠上以后宽字被盖掉一半，下一帧会留下残字。按钮留到最后。
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    seen: &Seen,
    screen: &Screen,
    spin: usize,
) -> (Rect, Vec<Spot>, Hits) {
    let focus = seen.focus();
    let count = seen.talk.board.shells_running();
    let button = (count > 0).then(|| {
        seen.config
            .text
            .jobs
            .button
            .replace("{count}", &count.to_string())
    });
    let button_w = button.as_ref().map_or(0, |b| b.width() + GAP);
    // 扩展包放的界面块：用量前面，一块一格，最先丢（蓝图「界面块」第 1、4 条）。
    let status = crate::ui::widget::status(seen);
    let mut cells: Vec<Part> = status
        .iter()
        .map(|(text, style, _)| Part::new(0, vec![Span::styled(text.clone(), *style)]))
        .collect();
    // 用量那一格（`usage_cell.rs`）：环、累计、花费；卡住的红字加「压缩」，正在压的转圈。
    let facts = usage_cell::Facts {
        t: &seen.talk.transcript,
        preview: seen.talk.preview,
        total: seen.talk.usage_total(),
        spent: seen.spent(),
        texts: &seen.config.text.usage_cell,
        spinner: &seen.config.timeline.spinner,
    };
    let named: Vec<(&str, Part)> = usage_cell::pieces(&facts, spin, hovered(screen))
        .into_iter()
        .map(|p| (FOOTER_RIGHT[0], Part::from(p)))
        .collect();
    // 照配置排右段的几件（「挂载位」第 4 条 SL-2）：藏起来的不写，一件几格的照原来的先后挨着。
    let (order, _) = arrange(&FOOTER_RIGHT, &seen.config.layout.footer);
    for (name, _) in order {
        cells.extend(
            named
                .iter()
                .filter(|(n, _)| *n == name)
                .map(|(_, p)| p.clone()),
        );
    }
    let widths: Vec<usize> = cells
        .iter()
        .map(|p| p.spans.iter().map(Span::width).sum())
        .collect();
    let cells_meta: Vec<Meta> = cells.iter().map(|p| (p.tight, p.hit)).collect();
    let (left, right, kept) = split(
        [
            left(seen, Model::Full, true),
            left(seen, Model::Short, true),
            left(seen, Model::Short, false),
            left(seen, Model::Bare, false),
        ],
        cells,
        button_w,
        area.width,
    );
    frame.render_widget(Paragraph::new(left.clone()), area);
    let right_w = right.width();
    frame.render_widget(Paragraph::new(right.right_aligned()), area);
    let (spots, hits) = status_spots(area, right_w, (&cells_meta, &widths, &kept), &status);
    let none = Rect::new(area.x, area.y, 0, 0);
    let Some(button) = button else {
        return (none, spots, hits);
    };
    let Some(x) = middle(
        left.width(),
        right_w,
        button.width(),
        usize::from(area.width),
    ) else {
        return (none, spots, hits);
    };
    let at = Rect::new(
        area.x + u16::try_from(x).unwrap_or(0),
        area.y,
        u16::try_from(button.width()).unwrap_or(0),
        1,
    );
    let style = button_style(screen, at, focus == Focus::Footer(Button::Background));
    frame.render_widget(Paragraph::new(Line::styled(button, style)), at);
    (at, spots, hits)
}

/// 用量那一格（点了开 `/context`）、「压缩」（点了同 `/compact`）在哪；没画的是空的。
#[derive(Debug, Clone, Copy, Default)]
pub struct Hits {
    pub usage: Rect,
    pub compact: Rect,
}

/// 指针停在用量那一格的哪一截上（照上一帧的位置）。
fn hovered(screen: &Screen) -> Option<Hit> {
    let p = screen.pointer?;
    if screen.areas.compact.contains(p) {
        Some(Hit::Compact)
    } else if screen.areas.usage_cell.contains(p) {
        Some(Hit::Usage)
    } else {
        None
    }
}

/// 一格和前一格只隔一个空格没有、点了做什么（[`Part`] 的 `tight`、`hit`）。
type Meta = (bool, Option<Hit>);

/// 底栏上界面块那几格、用量那一格在哪：右边那一段右对齐，留下的格子照先后排、之间 ` · `（`tight` 的一个空格）；界面块排在
/// 最前面（悬停时说是哪个软件放的）。
fn status_spots(
    area: Rect,
    right_w: usize,
    (meta, widths, kept): (&[Meta], &[usize], &[bool]),
    status: &[(String, ratatui::style::Style, &crate::widgets::Block)],
) -> (Vec<Spot>, Hits) {
    let mut x = usize::from(area.right()).saturating_sub(right_w);
    let mut spots = Vec::new();
    let mut hits = Hits::default();
    let mut first = true;
    let sep = crate::sheet::sep().width();
    for (i, (&w, _)) in widths
        .iter()
        .zip(kept)
        .enumerate()
        .filter(|(_, (_, k))| **k)
    {
        let (tight, hit) = meta.get(i).copied().unwrap_or_default();
        if !first {
            x += if tight { 1 } else { sep };
        }
        first = false;
        let rect = Rect::new(
            u16::try_from(x).unwrap_or(0),
            area.y,
            u16::try_from(w).unwrap_or(0),
            1,
        );
        if let Some((_, _, block)) = status.get(i) {
            spots.push(Spot {
                rect,
                package: block.package.clone(),
                id: block.id.clone(),
                what: Kind::Status,
            });
        }
        match hit {
            Some(Hit::Compact) => hits.compact = rect,
            Some(Hit::Usage) if hits.usage.width == 0 => hits.usage = rect,
            Some(Hit::Usage) => hits.usage = hits.usage.union(rect),
            None => {}
        }
        x += w;
    }
    (spots, hits)
}

/// 按钮的样子：强调色；悬停加粗；焦点在上面反色（第 1、2 条）。
fn button_style(screen: &Screen, at: Rect, focused: bool) -> ratatui::style::Style {
    if focused {
        return theme::selected();
    }
    let hovered = screen.pointer.is_some_and(|p| at.contains(p));
    let style = theme::accent();
    if hovered {
        style.add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        style
    }
}

/// 一行怎么分：左边从全的到窄的四种（`lefts`：全的；模型编号只写最后一段的；再把权限级别的字收掉、只留图标的；最后才
/// 收掉供应商名），右边的格子全放得下就用最宽的那种，空出的地方给右边；最窄的也放不下右边全部时再照 [`choose`] 挑格子
/// （2026-10-11 项目主人定：原来模型编号一长，速度、累计先丢了，中间空着一大截；2026-10-18 项目主人：窄了「开放权限」这些字
/// 可以收，图标留着；2026-10-23 项目主人：供应商名最后再收，不然不知道用的是哪家）。
/// `button` 是后台按钮连两边的空占几列。交回左边、右边。
fn split(
    lefts: [Vec<Span<'_>>; 4],
    cells: Vec<Part>,
    button: usize,
    width: u16,
) -> (Line<'static>, Line<'static>, Vec<bool>) {
    let need = joined_width(&cells);
    // 界面块、速度、花费这几格（`keep` 不过 [`EARLY`]）比权限级别的字先丢（2026-10-19 项目主人）：丢了它们放得下的，
    // 照旧写字。
    let late: Vec<&Part> = cells.iter().filter(|p| p.keep > EARLY).collect();
    let need_late = joined_width_of(&late);
    let fits = |spans: &Vec<Span<'_>>, need: usize| {
        spans.iter().map(Span::width).sum::<usize>() + GAP + button + need <= usize::from(width)
    };
    let [full, short, iconic, bare] = lefts;
    let chosen = if fits(&full, need) {
        full
    } else if fits(&short, need_late) {
        short
    } else if fits(&iconic, need_late) {
        iconic
    } else {
        bare
    };
    let left = left_line(chosen, width);
    let room = usize::from(width).saturating_sub(left.width() + GAP + button);
    let kept = choose(&cells, room);
    let right = Line::from(join(cells, &kept));
    (left, right, kept)
}

/// 这几格照 ` · ` 连起来有几列。
fn joined_width(parts: &[Part]) -> usize {
    joined_width_of(&parts.iter().collect::<Vec<_>>())
}

/// 同 [`joined_width`]，给挑出来的几格。
fn joined_width_of(parts: &[&Part]) -> usize {
    parts
        .iter()
        .enumerate()
        .map(|(i, p)| gap_before(p, i == 0) + p.spans.iter().map(Span::width).sum::<usize>())
        .sum()
}

/// `keep` 不过它的格子（界面块、速度、花费）比权限级别的字先丢。
const EARLY: u8 = 2;

/// 按钮从第几列起：左右两段之间的空当（两边各留 `GAP`）正中；放不下是 `None`。
fn middle(left: usize, right: usize, button: usize, width: usize) -> Option<usize> {
    let start = left + GAP;
    let end = width.checked_sub(right + GAP)?;
    let gap = end.checked_sub(start)?;
    (gap >= button).then(|| start + (gap - button) / 2)
}

/// 右边的一格：放不下时先丢 `keep` 小的；`tight` 的和前一格只隔一个空格（用量那一格卡住时的几截）；`hit` 是点了做什么。
#[derive(Clone)]
struct Part {
    keep: u8,
    tight: bool,
    hit: Option<Hit>,
    spans: Vec<Span<'static>>,
}

impl Part {
    fn new(keep: u8, spans: Vec<Span<'static>>) -> Self {
        Self {
            keep,
            tight: false,
            hit: None,
            spans,
        }
    }
}

impl From<usage_cell::Piece> for Part {
    fn from(p: usage_cell::Piece) -> Self {
        Self {
            keep: p.keep,
            tight: p.tight,
            hit: p.hit,
            spans: p.spans,
        }
    }
}

/// 这一格前面隔几列：头一格不隔，`tight` 的一个空格，别的 ` · `。
fn gap_before(part: &Part, first: bool) -> usize {
    match (first, part.tight) {
        (true, _) => 0,
        (false, true) => 1,
        (false, false) => crate::sheet::sep().width(),
    }
}

/// 照 `keep` 从大到小挑，放得进 `room` 列的留下、放不下的跳过（后面短的照样能进：2026-10-11 项目主人报，长的那一格
/// 放不下时，放得下的累计跟着丢了）；留下的照原来的先后，用 ` · ` 隔开。
#[cfg(test)]
fn fit(parts: Vec<Part>, room: usize) -> Vec<Span<'static>> {
    let kept = choose(&parts, room);
    join(parts, &kept)
}

/// 照 `keep` 从大到小挑：哪几格留下。
fn choose(parts: &[Part], room: usize) -> Vec<bool> {
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(parts[i].keep));
    let mut kept = vec![false; parts.len()];
    for i in order {
        kept[i] = true;
        let chosen: Vec<&Part> = parts
            .iter()
            .zip(&kept)
            .filter(|(_, k)| **k)
            .map(|(p, _)| p)
            .collect();
        if joined_width_of(&chosen) > room {
            kept[i] = false;
        }
    }
    kept
}

/// 留下的照原来的先后，用 ` · ` 隔开。
fn join(parts: Vec<Part>, kept: &[bool]) -> Vec<Span<'static>> {
    let parts = parts
        .into_iter()
        .zip(kept)
        .filter(|(_, k)| **k)
        .map(|(p, _)| p);
    let mut spans = Vec::new();
    for (i, part) in parts.enumerate() {
        match gap_before(&part, i == 0) {
            0 => {}
            1 => spans.push(Span::raw(" ")),
            _ => spans.push(Span::styled(crate::sheet::sep(), theme::dim())),
        }
        spans.extend(part.spans);
    }
    spans
}

/// 左边那一段：放不下截掉加 `…`（`tui.md`「框下面那一行」）。
fn left_line(spans: Vec<Span<'_>>, width: u16) -> Line<'static> {
    let owned = spans
        .into_iter()
        .map(|s| Span::styled(s.content.into_owned(), s.style))
        .collect();
    Line::from(super::panel::clip_spans(owned, usize::from(width)))
}

/// 模型那一格写多全（[`split`] 从全的往窄里挑）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Model {
    /// 模型编号接暗色的供应商名。
    Full,
    /// 模型编号只写最后一段（`clinepass/cline-pass/deepseek-v4.1-flash` 写 `deepseek-v4.1-flash`），供应商名照写。
    Short,
    /// 只写最后一段，供应商名也收掉（最窄的时候）。
    Bare,
}

/// 左边那一段：权限级别、模型和端点、思考强度。`model`：模型那一格写多全；`named`：权限级别写字，不写的只留图标。一行放不下
/// 时用窄的（[`split`]）。
fn left<'a>(seen: &Seen<'a>, model: Model, named: bool) -> Vec<Span<'a>> {
    let text = &seen.config.text;
    let t = &seen.talk.transcript;
    match &t.link {
        Link::Connecting => vec![Span::styled(text.connecting.as_str(), theme::dim())],
        Link::Reconnecting => vec![Span::styled(text.reconnecting.as_str(), theme::warn())],
        // 整句红字折行写在输入框上面（`above.rs`），这里只写前面那一截。
        Link::Down(reason) => vec![Span::styled(super::above::head(reason), theme::error())],
        Link::Ready => {
            let level = t.level;
            let name = text.levels.get(&level).map_or("", String::as_str);
            let icon = seen.config.layout.level_icons.get(&level);
            let (label, sep) = level_label(icon.map(String::as_str), name, named);
            let level_part = vec![Span::styled(label, theme::level(level))];
            let mut spans = Vec::new();
            // 都在冷却：模型那一格变黄写原因，到点了变回原样（「配置与模型」第 7 条）。
            let now = jiff::Timestamp::now();
            let cooling = t
                .cooling()
                .and_then(|until| cooling_word(until, now, &text.models));
            if let Some(word) = cooling {
                spans.push(Span::styled(word, theme::warn()));
            } else if let Some((shown, endpoint)) = t.shown_model() {
                // 固定的三个池写 `@旗舰池`（`pools.rs`，2026-11-09 项目主人：选了以后底栏不写 `@flagship`）
                let shown = crate::pools::label(&shown, &text.settings.pool_names).unwrap_or(shown);
                spans.extend(model_spans(shown, &endpoint, model));
                // 思考强度（`/effort`）：什么都不发的不写。
                if let Some(effort) = t.effort() {
                    // 强调色（2026-10-02 项目主人：和「工作区」那个蓝一样）。
                    spans.push(Span::styled(crate::sheet::sep(), theme::dim()));
                    spans.push(Span::styled(effort.to_string(), theme::accent()));
                }
            }
            arranged(&seen.config.layout.footer, (level_part, sep), spans)
        }
    }
}

/// 照配置排左段的两件（「挂载位」第 4 条 SL-2）：藏起来的、空的不写；权限级别后面接它自己的分隔（只留图标时是一个空格），
/// 别的之间 ` · `。
fn arranged<'a>(
    config: &[Slot],
    (level, level_sep): (Vec<Span<'a>>, String),
    model: Vec<Span<'a>>,
) -> Vec<Span<'a>> {
    let (order, _) = arrange(&FOOTER_LEFT, config);
    let mut parts = [Some(level), Some(model)];
    let mut out = Vec::new();
    // 前一件是不是权限级别；还没写的是 `None`。
    let mut after_level = None;
    for (name, _) in order {
        let at = usize::from(name == FOOTER_LEFT[1]);
        let Some(part) = parts[at].take().filter(|p| !p.is_empty()) else {
            continue;
        };
        if let Some(level) = after_level {
            let sep = if level {
                level_sep.clone()
            } else {
                crate::sheet::sep()
            };
            out.push(Span::styled(sep, theme::dim()));
        }
        after_level = Some(at == 0);
        out.extend(part);
    }
    out
}

/// 模型那一格：全的写模型编号接暗色的供应商名；窄的先只写编号最后一段、供应商名照写，最窄才收掉供应商名（2026-10-19 项目主人：
/// 宽度不够时供应商信息也可以隐藏；2026-10-23 改：「最后再缩，而不是一开始就缩」，同名的模型分不出是哪家）。
fn model_spans(model: String, endpoint: &str, how: Model) -> Vec<Span<'static>> {
    let name = match how {
        Model::Full => model,
        Model::Short | Model::Bare => model.rsplit('/').next().unwrap_or(&model).to_string(),
    };
    let mut spans = vec![Span::styled(name, theme::model())];
    if how != Model::Bare {
        spans.push(Span::styled(format!(" {endpoint}"), theme::dim()));
    }
    spans
}

/// 权限级别那一格写什么、后面接模型那一格用什么隔开：写字的是「⏵⏵ 开放权限」接 ` · `；窄了只留图标的是「⏵⏵」接一个空格，
/// 不留 ` · `（2026-10-19 项目主人：字收掉了，那个点也收掉）。没有图标的照旧写字。
fn level_label(icon: Option<&str>, name: &str, named: bool) -> (String, String) {
    match icon {
        Some(icon) if !named => (icon.trim_end().to_string(), " ".to_string()),
        _ => (format!("{}{name}", icon.unwrap_or("")), crate::sheet::sep()),
    }
}

/// 都在冷却时模型那一格写什么：知道几时恢复的写还要几分钟（向上取整），不知道的只写都在冷却；到点了是 `None`。
fn cooling_word(
    until: Option<jiff::Timestamp>,
    now: jiff::Timestamp,
    texts: &crate::config::ModelTexts,
) -> Option<String> {
    let Some(until) = until else {
        return Some(texts.cooling.clone());
    };
    let left = until.duration_since(now).as_secs();
    if left <= 0 {
        return None;
    }
    let minutes = (left + 59) / 60;
    Some(texts.cooling_until.replace("{n}", &minutes.to_string()))
}

#[cfg(test)]
mod tests;
