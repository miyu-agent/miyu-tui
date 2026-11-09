//! 侧边栏挂的出厂部件（蓝图 `tui.md`「挂载位」第 4 条，SL-1）：头像、会话、工作目录、上下文、用量、循环、最近会话，照这个
//! 先后。每件照宽度排出自己那几行；要点的（短编号、最近会话）画好以后照给它的那一块记点击区。

use ratatui::Frame;
use ratatui::layout::Rect;

use super::super::layout::slot::{Laid, Part};
use crate::app::{MascotState, Place, Screen, Seen};

use crate::config::slots::SIDE_PARTS as NAMES;

/// 侧边栏画的时候要用的。
pub struct Side<'s, 'a> {
    /// 这一帧看到的。
    pub seen: &'s Seen<'a>,
    /// 点击区、最近会话那几行记在这里。
    pub screen: &'s mut Screen,
    /// 吉祥物的动作。
    pub pet: &'s mut MascotState,
}

/// `avatar`：人格的头像，没有头像的吉祥物（照旧受「侧边栏显示形象」管），居中。
pub struct Avatar;

impl Part<Side<'_, '_>> for Avatar {
    fn name(&self) -> &'static str {
        NAMES[0]
    }

    fn lay(&mut self, side: &Side, _: u16) -> Laid {
        let shown = super::super::avatar_view::slot_shown(side.seen, Place::Sidebar);
        Laid::Rows(if shown {
            side.seen.config.mascot.rows
        } else {
            0
        })
    }

    fn placed(&mut self, frame: &mut Frame, area: Rect, _: (usize, usize), side: &mut Side) {
        // 照侧边栏整栏居中：给的这一块左右各少一列（和竖线、右边留的那一列），照回整栏算。
        let whole = Rect::new(
            area.x.saturating_sub(1),
            area.y,
            area.width + 2,
            area.height,
        );
        let cols = side.seen.config.mascot.cols.min(whole.width);
        let rect = Rect::new(
            whole.x + (whole.width - cols) / 2,
            area.y,
            cols,
            area.height,
        );
        let parts = (side.seen, &*side.screen, &mut *side.pet);
        super::super::avatar_view::draw(frame, rect, parts, Place::Sidebar, true);
    }
}

/// `session`：会话名、短编号、人格、预设。点短编号复制完整编号。
#[derive(Default)]
pub struct Session {
    id_row: Option<usize>,
}

impl Part<Side<'_, '_>> for Session {
    fn name(&self) -> &'static str {
        NAMES[1]
    }

    fn lay(&mut self, side: &Side, width: u16) -> Laid {
        let t = &side.seen.talk.transcript;
        let (lines, id_row) =
            super::session_lines(t, side.seen.config, width, side.seen.session_picks());
        self.id_row = id_row;
        Laid::Lines(lines)
    }

    fn placed(
        &mut self,
        _: &mut Frame,
        area: Rect,
        (first, shown): (usize, usize),
        side: &mut Side,
    ) {
        // 短编号那一行露出来了才能点（往下滚过的照露出来的那一截算）。
        if let Some(row) = self
            .id_row
            .filter(|row| (first..first + shown).contains(row))
        {
            let y = area.y + u16::try_from(row - first).unwrap_or(0);
            side.screen.areas.session_id = Rect::new(area.x, y, area.width, 1);
        }
    }
}

/// `cwd`：工作目录照会话的（核心 9-7 上）；还没开会话、核心旧的照终端所在的。
pub struct Cwd;

impl Part<Side<'_, '_>> for Cwd {
    fn name(&self) -> &'static str {
        NAMES[2]
    }

    fn lay(&mut self, side: &Side, width: u16) -> Laid {
        Laid::Lines(super::cwd_lines(
            side.seen.config,
            width,
            side.seen.current_workspace(),
        ))
    }

    /// 第 0 行是标题，第 1 行起是路径：第 2 行起都是路径放不下折下来的。
    fn continues(&self, row: usize) -> bool {
        row >= 2
    }
}

/// `context`：上下文。
pub struct Context;

impl Part<Side<'_, '_>> for Context {
    fn name(&self) -> &'static str {
        NAMES[3]
    }

    fn lay(&mut self, side: &Side, width: u16) -> Laid {
        Laid::Lines(super::context_lines(
            &side.seen.talk.transcript,
            side.seen.config,
            width,
        ))
    }
}

/// `usage`：用量，最后接花了多少（核心 8-15）。
pub struct Usage;

impl Part<Side<'_, '_>> for Usage {
    fn name(&self) -> &'static str {
        NAMES[4]
    }

    fn lay(&mut self, side: &Side, _: u16) -> Laid {
        let seen = side.seen;
        let total = seen.talk.usage_total();
        let mut lines = super::usage_lines(&seen.talk.transcript, &total, seen.config);
        lines.extend(super::spent_lines(seen.spent(), seen.config));
        Laid::Lines(lines)
    }
}

/// `loops`：这个会话挂着的循环（「循环」第 3 条）。
pub struct Loops;

impl Part<Side<'_, '_>> for Loops {
    fn name(&self) -> &'static str {
        NAMES[5]
    }

    fn lay(&mut self, side: &Side, width: u16) -> Laid {
        let seen = side.seen;
        let at_now = (jiff::Timestamp::now(), &jiff::tz::TimeZone::system());
        let mine =
            super::super::loop_tag::of(&seen.link.loops, seen.talk.transcript.session.as_deref());
        Laid::Lines(super::super::loop_tag::sidebar(
            &mine,
            seen.config,
            at_now,
            width,
        ))
    }
}

/// `recent`：最近会话，一行一个；点一行切过去。
#[derive(Default)]
pub struct Recent {
    ids: Vec<Option<String>>,
}

impl Part<Side<'_, '_>> for Recent {
    fn name(&self) -> &'static str {
        NAMES[7]
    }

    fn lay(&mut self, side: &Side, width: u16) -> Laid {
        let seen = side.seen;
        let current = seen.talk.listed_current();
        let list = seen.talk.sessions_seen.as_deref().unwrap_or_default();
        let count = seen.config.layout.sidebar_sessions;
        let (mut lines, ids) = super::super::recent::recent_lines(
            (list, &seen.talk.unseen),
            current,
            count,
            (width, super::super::spin_frame(seen)),
            seen.config,
        );
        // 记号挂在缩进那两格里，名字和别的几段的内容对齐（2026-10-27 项目主人：「最近会话的缩进有待优化」）：只给标题上色。
        if let Some(title) = lines.first_mut() {
            *title = std::mem::take(title).style(crate::theme::side_title());
        }
        self.ids = ids;
        Laid::Lines(lines)
    }

    fn placed(
        &mut self,
        _: &mut Frame,
        area: Rect,
        (first, shown): (usize, usize),
        side: &mut Side,
    ) {
        // 标题那一行（第 0 行）不算：从第一个露出来的会话起一行一个（往下滚过、标题滚出去的从第 `first` 行起）。
        let start = first.max(1);
        let rows = (first + shown).saturating_sub(start);
        side.screen.areas.recent = Rect {
            y: area.y + u16::try_from(start - first).unwrap_or(0),
            height: u16::try_from(rows).unwrap_or(0),
            ..area
        };
        side.screen.recent = self
            .ids
            .iter()
            .skip(start)
            .take(rows)
            .flatten()
            .cloned()
            .collect();
    }
}

/// `jump`：跳转消息（排期第 15 条）：这个会话里你说的话，列最近的 `sidebar_jumps` 条，新的在下（发版前 check 第 18 条，2026-11-09
/// 项目主人：正文里新的在底下、越往上越旧，这里跟着一样；原来新的在上），一条一行截掉放不下的；点一行同 `/jump` 选它，
/// 顶到视口第一行（「跳到说过的话」第 3 条）。
#[derive(Default)]
pub struct Jumps {
    ids: Vec<u64>,
    /// 读进来的比要列的少（老会话还有更早的页没读，发版前 check 第 33 条）。
    pub short: bool,
}

impl Part<Side<'_, '_>> for Jumps {
    fn name(&self) -> &'static str {
        NAMES[6]
    }

    fn lay(&mut self, side: &Side, width: u16) -> Laid {
        let seen = side.seen;
        let count = seen.config.layout.sidebar_jumps;
        let mut spoken: Vec<&crate::transcript::Entry> = seen
            .talk
            .transcript
            .entries
            .iter()
            .rev()
            .filter(|e| e.kind == crate::transcript::Kind::User && !e.hidden)
            .take(count)
            .collect();
        // 取的是最近的几条，排成旧的在上、新的在下（同正文）
        spoken.reverse();
        self.short = spoken.len() < count;
        self.ids = spoken.iter().map(|e| e.id).collect();
        if spoken.is_empty() {
            return Laid::Lines(Vec::new());
        }
        let room = width.saturating_sub(2);
        let mut lines = vec![ratatui::text::Line::styled(
            seen.config.text.side_jumps.clone(),
            crate::theme::side_title(),
        )];
        // 当前这一句的箭头照主题的 `marks.pointer`（两格宽），别的行空同样宽。
        let pointer = crate::sheet::read(|s| s.marks.pointer.clone());
        let blank = " ".repeat(unicode_width::UnicodeWidthStr::width(pointer.as_str()));
        lines.extend(spoken.iter().map(|e| {
            let said = e.listed();
            let first = said
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or_default();
            let text = super::super::rows::clip(first.trim(), room);
            // 当前这一句打头写箭头（排期第 44 条，2026-11-04 项目主人：「箭头。不要用圆点。」）。
            let lead = if side.screen.current_said == Some(e.id) {
                ratatui::text::Span::styled(pointer.clone(), crate::theme::accent())
            } else {
                ratatui::text::Span::raw(blank.clone())
            };
            ratatui::text::Line::from(vec![
                lead,
                ratatui::text::Span::styled(text, crate::theme::dim()),
            ])
        }));
        Laid::Lines(lines)
    }

    fn placed(
        &mut self,
        _: &mut Frame,
        area: Rect,
        (first, shown): (usize, usize),
        side: &mut Side,
    ) {
        // 标题那一行（第 0 行）不算，同最近会话。
        let start = first.max(1);
        let rows = (first + shown).saturating_sub(start);
        side.screen.areas.jumps = Rect {
            y: area.y + u16::try_from(start - first).unwrap_or(0),
            height: u16::try_from(rows).unwrap_or(0),
            ..area
        };
        side.screen.jumps = self
            .ids
            .iter()
            .skip(start - 1)
            .take(rows)
            .copied()
            .collect();
    }
}

/// 这一处的几件，照出厂的先后。
pub fn all() -> (Avatar, Session, Cwd, Context, Usage, Loops, Recent, Jumps) {
    (
        Avatar,
        Session::default(),
        Cwd,
        Context,
        Usage,
        Loops,
        Recent::default(),
        Jumps::default(),
    )
}
