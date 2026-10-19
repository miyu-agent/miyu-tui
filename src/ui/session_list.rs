//! 会话列表的框（蓝图 `tui.md`「会话列表 `/sessions`」第 1、2 条）：和输入历史列表一个位置、一个样子。上边框写「会话」、
//! 几个、打的字；一行一个会话：置顶的打头一个记号，标题（没起名的暗色），暗色短编号，有工作目录的接着写；右边写
//! 多久以前有过动静。勾上的行首一个紫色的勾（主题的 `picked`，2026-10-01 项目主人要紫色）。选中的 `❯` 那一列写一个记号：
//! 正在用的强调色的 `●`，在跑的转着的盲文（照时间线的转圈，正在用的转圈也是强调色），别的空着（2026-10-02 项目主人：在跑的
//! 看不出来，右边的「当前」不够显眼；2026-10-16 起不另占一列，左边不再空一大块）。

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use serde::Deserialize;

use super::panel::{self, Chrome, Row};

mod groups;
use crate::config::Config;
use crate::core::SessionInfo;
use crate::session_list::{SessionList, short};
use crate::theme;
use unicode_width::UnicodeWidthStr;

/// 框里的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 上边框写的。
    pub title: String,
    /// 几个，`{count}`。
    pub count: String,
    /// 打了字时接在后面的。
    pub query: String,
    /// 下边框的按键提示。
    pub hint: String,
    /// 核心还没交回来。
    pub loading: String,
    /// 一个都对不上。
    pub empty: String,
    /// 多久以前：一分钟以内、几分钟、几小时、几天。
    pub now: String,
    /// `{n}` 分钟前。
    pub minutes: String,
    /// `{n}` 小时前。
    pub hours: String,
    /// `{n}` 天前。
    pub days: String,
    /// 勾了几个，`{count}`。
    pub ticked: String,
    /// 按了一次 `Ctrl+D`，`{title}`。
    pub delete_again: String,
    /// 按了一次 `Ctrl+D`、要删好几个，`{count}`。
    pub delete_many: String,
    /// 删了好几个，`{count}`。
    pub deleted_many: String,
    /// 删了，`{title}`。
    pub deleted: String,
    /// 有确认、提问在等你回答的会话，右边写的（「确认和提问的抽屉」第 10 条）。
    pub asking: String,
    /// 正在用的不能删。
    pub delete_current: String,
    /// 置顶了、取消了。
    pub pinned: String,
    /// 取消置顶了。
    pub unpinned: String,
    /// 段头（第 10 条）：最近、近 7 天、近 30 天、30 天前。
    pub groups: [String; 4],
    /// 编号后面那一列：一共用了多少 token，`{tokens}`。
    pub tokens: String,
}

/// 哪个会话是正在用的，哪几个有确认、提问在等你（「确认和提问的抽屉」第 10 条）。
#[derive(Debug, Clone, Copy, Default)]
pub struct Marks<'a> {
    /// 正在用的会话。
    pub current: Option<&'a str>,
    /// 在问你的会话。
    pub asking: &'a [String],
}

/// 框（标题、提示）、排好的行、每一行是对得上的第几个；`marks.current` 是正在用的会话，放不下 `max` 行时离选中的远的少露。
/// `frame` 是转圈转到第几帧。
pub fn lines(
    list: &SessionList,
    marks: Marks<'_>,
    here: &str,
    config: &Config,
    width: u16,
    max: usize,
    frame: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    let texts = &config.text.sessions;
    let found = list.matches();
    let mut meta = vec![Span::styled(
        texts.count.replace("{count}", &found.len().to_string()),
        theme::dim(),
    )];
    if !list.query.is_empty() {
        meta.push(Span::styled(texts.query.clone(), theme::dim()));
        meta.push(Span::styled(
            list.query.clone(),
            Style::new().fg(Color::Reset),
        ));
    }
    if !list.ticked.is_empty() {
        let count = list.ticked.len().to_string();
        meta.push(Span::styled(
            texts.ticked.replace("{count}", &count),
            theme::dim(),
        ));
    }
    let chrome = Chrome::new(&texts.title, meta).hint(&texts.hint);
    let note = |text: &str| {
        let line = panel::item(
            false,
            vec![Span::styled(text.to_string(), theme::faint())],
            None,
            width,
        );
        (Vec::from([line]), vec![None])
    };
    if !list.loaded || found.is_empty() {
        let (lines, map) = note(if list.loaded {
            &texts.empty
        } else {
            &texts.loading
        });
        return (chrome, lines, map);
    }
    let now = jiff::Timestamp::now();
    // 最多露 `session_rows` 行，选中的停在正中间（第 1 条）。
    let shown = config.layout.session_rows.max(1);
    let top = crate::menu::top(list.selected, list.pinned, found.len(), shown);
    let ticking = !list.ticked.is_empty();
    // 标题一列多宽：对得上的里最长的那个（连置顶的记号），最宽 `session_title_width`（第 1 条）。
    let column = found
        .iter()
        .map(|info| head_width(info, config))
        .max()
        .unwrap_or(0)
        .min(config.layout.session_title_width);
    // 编号后面的总 token 一列：靠右对齐，照露出来的里最宽的那个（2026-10-19 项目主人）。
    let tokens = |info: &SessionInfo| {
        texts
            .tokens
            .replace("{tokens}", &crate::meter::short(info.tokens))
    };
    let tokens_width = found
        .iter()
        .skip(top)
        .take(shown)
        .map(|info| tokens(info).width())
        .max()
        .unwrap_or(0);
    // 放不下时标题先让：编号、总 token 一列、右边那一格照全写，标题一列截短（最少留 `TITLE_MIN` 列；2026-10-19 真数据上
    // 看到窄的框里总 token 被右边的目录挤成「Σ4…」）。
    let right_of = |info: &SessionInfo| {
        if marks.asking.contains(&info.session) {
            texts.asking.width()
        } else {
            right(info, marks.current, here, now, config).width()
        }
    };
    let widest_right = found
        .iter()
        .skip(top)
        .take(shown)
        .map(|info| right_of(info))
        .max()
        .unwrap_or(0);
    let tick_width = if ticking {
        config.layout.tick_mark.width()
    } else {
        0
    };
    // 选中的记号两列、右边那一格前面空一列、编号「  #」加 8 个字符、总 token 前面空两列。
    let fixed = 2 + widest_right + 1 + tick_width + 3 + 8 + 2 + tokens_width;
    let column = column.min(
        usize::from(width)
            .saturating_sub(fixed)
            .max(column.min(TITLE_MIN)),
    );
    // 没打字时照最近动静分段，段前一行段头（第 10 条）；露出来的一截从一段中间开始的也先写段头。
    let tz = jiff::tz::TimeZone::system();
    let today = now.to_zoned(tz.clone()).date();
    let grouped = list.query.is_empty();
    let mut last_group = None;
    let mut rows: Vec<Row> = Vec::new();
    for (i, info) in found.iter().enumerate().skip(top).take(shown) {
        if grouped {
            let group = groups::group(info, marks.current, today, &tz);
            if let Some(age) = group.filter(|_| group != last_group) {
                rows.push((None, groups::heading(&texts.groups[age.index()])));
            }
            last_group = group;
        }
        rows.push({
            let mut content = Vec::new();
            if ticking {
                let mark = &config.layout.tick_mark;
                let mark = if list.ticked.contains(&info.session) {
                    mark.clone()
                } else {
                    " ".repeat(mark.width())
                };
                content.push(Span::styled(mark, theme::picked()));
            }
            content.extend(head(info, config, column));
            content.push(Span::styled(
                format!("  #{}", short(&info.session)),
                theme::faint(),
            ));
            content.push(Span::styled(
                format!("  {:>tokens_width$}", tokens(info)),
                theme::faint(),
            ));
            let Marks { current, asking } = marks;
            let right = if asking.contains(&info.session) {
                // 这个会话在问你（「确认和提问的抽屉」第 10 条）：切过去就弹。
                Span::styled(texts.asking.clone(), theme::accent())
            } else {
                Span::styled(right(info, current, here, now, config), theme::faint())
            };
            let lead = marker(info, current, frame, config);
            let line = panel::item_with(i == list.selected, lead, content, Some(right), width);
            (Some(i), line)
        });
    }
    let rows = groups::drop_orphans(panel::fit(rows, Some(list.selected), max));
    let (map, lines) = rows.into_iter().unzip();
    (chrome, lines, map)
}

/// 框窄了标题一列最少留几列。
const TITLE_MIN: usize = 8;

/// 标题前面的记号，写在选中的 `❯` 那一列（不另占一列，2026-10-16 项目主人）：在跑的转圈，选中了也转；正在用的 `●`，
/// 选中了让给 `❯`（选中的那一行整行铺了底）；都是的转圈、强调色；别的没有。
fn marker(
    info: &SessionInfo,
    current: Option<&str>,
    frame: usize,
    config: &Config,
) -> Option<(Span<'static>, bool)> {
    let spinner = &config.timeline.spinner;
    let is_current = current == Some(info.session.as_str());
    let mark = if info.busy {
        spinner[frame % spinner.len().max(1)].clone()
    } else if is_current {
        config.layout.current_mark.clone()
    } else {
        return None;
    };
    let style = if is_current {
        theme::accent()
    } else {
        theme::dim()
    };
    Some((Span::styled(format!("{mark} "), style), info.busy))
}

/// 标题那一列有多宽：置顶的记号加标题（没起名的写「未命名会话」）。
fn head_width(info: &SessionInfo, config: &Config) -> usize {
    let pin = if info.pinned {
        config.icons.pin.width()
    } else {
        0
    };
    pin + title(info, config).width()
}

/// 标题（没起名的写「未命名会话」，和起了名的一个颜色：2026-10-01 项目主人）。
fn title<'a>(info: &'a SessionInfo, config: &'a Config) -> &'a str {
    // 没起名的写核心给的预览（第一句话开头，核心 9-5），都没有的才写「未命名会话」。
    info.title
        .as_deref()
        .or(info.preview.as_deref())
        .unwrap_or(&config.text.untitled)
}

/// 标题那一列：置顶的记号、标题，截到 `column` 列、补空格对齐。
fn head(info: &SessionInfo, config: &Config, column: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut room = column;
    if info.pinned {
        spans.push(Span::styled(config.icons.pin.clone(), theme::accent()));
        room = room.saturating_sub(config.icons.pin.width());
    }
    let text = super::rows::clip(title(info, config), u16::try_from(room).unwrap_or(u16::MAX));
    let pad = room.saturating_sub(text.width());
    spans.push(Span::raw(format!("{text}{}", " ".repeat(pad))));
    spans
}

/// 右边那一格：工作目录（和界面现在所在的目录 `here` 不一样时才写），再写「当前」「在忙」或者多久以前。
fn right(
    info: &SessionInfo,
    current: Option<&str>,
    here: &str,
    now: jiff::Timestamp,
    config: &Config,
) -> String {
    let texts = &config.text.sessions;
    // 正在用的、在跑的标题前面有记号，右边不再写「当前」「在忙」。
    let when = if current == Some(info.session.as_str()) || info.busy {
        String::new()
    } else {
        info.last_active
            .map(|at| ago(at, now, texts))
            .unwrap_or_default()
    };
    let elsewhere = info
        .cwd
        .as_deref()
        .filter(|cwd| crate::local::home_short(cwd) != here)
        .map(|cwd| short_path(cwd, config.layout.session_cwd_width));
    match elsewhere {
        Some(cwd) if when.is_empty() => cwd,
        Some(cwd) => format!("{cwd}  {when}"),
        None => when,
    }
}

/// 工作目录写短：家目录写成 `~`，长过 `width` 列的只写最后两层、前面 `…/`（第 1 条）。
fn short_path(cwd: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    let home = std::env::var("HOME").unwrap_or_default();
    let cwd = match cwd.strip_prefix(home.as_str()) {
        Some(rest) if !home.is_empty() && (rest.is_empty() || rest.starts_with('/')) => {
            format!("~{rest}")
        }
        _ => cwd.to_string(),
    };
    if cwd.width() <= width {
        return cwd;
    }
    let parts: Vec<&str> = cwd.trim_end_matches('/').rsplit('/').take(2).collect();
    let tail: Vec<&str> = parts.into_iter().rev().collect();
    format!("…/{}", tail.join("/"))
}

/// 多久以前有过动静。
fn ago(at: jiff::Timestamp, now: jiff::Timestamp, texts: &Texts) -> String {
    let secs = now.duration_since(at).as_secs().max(0);
    let n = |d: i64, words: &str| words.replace("{n}", &(secs / d).to_string());
    match secs {
        0..60 => texts.now.clone(),
        60..3600 => n(60, &texts.minutes),
        3600..86400 => n(3600, &texts.hours),
        _ => n(86400, &texts.days),
    }
}

#[cfg(test)]
mod tests;
