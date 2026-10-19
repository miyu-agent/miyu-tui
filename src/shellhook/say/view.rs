//! 这一轮排成哪几行（图纸 `shellhook.md`「印什么」）：只排你这一句后面的，照终端界面的排法（`ui/rows.rs` 的
//! `entry_rows`），你说的那句 fish 已经重放过了，不再排。

use std::cell::RefCell;

use ratatui::text::Line;

use crate::config::Config;
use crate::diagrams::Diagrams;
use crate::figures::Figures;
use crate::human::Human;
use crate::link_cards::LinkCards;
use crate::transcript::Transcript;
use crate::ui::rows::{Ctx, MdCache, Row, entry_rows, shown};

/// 排版要的几样：Markdown 的缓存、图、链接卡片、mermaid 图。shell 里不显示图（还没做），图那几样照空的。
pub struct Media {
    md: RefCell<MdCache>,
    figures: RefCell<Figures>,
    cards: RefCell<LinkCards>,
    diagrams: RefCell<Diagrams>,
}

impl Media {
    /// 照配置备好。
    pub fn new(config: &Config) -> Self {
        Self {
            md: RefCell::new(MdCache::new(config.layout.markdown_cache)),
            figures: RefCell::new(Figures::start(None, &config.figures, None, |_| true)),
            cards: RefCell::default(),
            diagrams: RefCell::default(),
        }
    }
}

/// 照这几样排。
pub struct View<'a> {
    pub config: &'a Config,
    pub human: &'a Human,
    pub media: &'a Media,
    /// 后台任务、待办。
    pub board: &'a crate::jobs::Board,
    /// 终端多宽。
    pub width: u16,
    /// 终端多高：图最多占几行照它。
    pub height: u16,
    /// 转圈转到第几帧。
    pub frame: usize,
}

impl View<'_> {
    /// 这一轮（`ours`，还没开轮的是 `None`）的几条排成行，一条之间空一行，和全屏一样；你说的那句 fish 已经重放过了，
    /// 不排。还在等她的第一个字的，末尾转着。
    pub fn rows(&self, transcript: &Transcript, ours: Option<u64>) -> Vec<Row> {
        let ctx = self.ctx(transcript);
        let mut rows = Vec::new();
        let theirs = |e: &crate::transcript::Entry| {
            e.kind != crate::transcript::Kind::User && e.turn.zip(ours).is_some_and(|(t, o)| t >= o)
        };
        for (i, entry) in transcript
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| theirs(e) && shown(e))
        {
            if !rows.is_empty() {
                rows.push(ctx.row(ctx.blank_slot(), Vec::new()));
            }
            rows.extend(entry_rows(i, entry, &ctx));
        }
        if transcript.waiting() {
            rows.extend(crate::ui::tail_rows(&ctx));
        }
        rows
    }

    /// 排好的行只要画出来的样子，前面空一行和提示符那一行隔开（2026-10-20 项目主人验收）。展示的卡片在 shell 里点不开：
    /// 「点击打开」那一行换成文件的路径（同一天项目主人：点不开的不必印）。`todos` 时末尾接待办。
    pub fn lines(
        &self,
        transcript: &Transcript,
        ours: Option<u64>,
        todos: bool,
    ) -> Vec<Line<'static>> {
        let open = &self.config.text.shown.open;
        let mut lines: Vec<Line<'static>> = Vec::new();
        for row in self.rows(transcript, ours) {
            if row.plain.trim() == open.trim() {
                if let Some(path) = shown_path(transcript, row.target) {
                    let indent = " ".repeat(usize::from(row.content_x));
                    lines.push(Line::styled(format!("{indent}{path}"), crate::theme::dim()));
                }
                continue;
            }
            lines.push(row.line);
        }
        while lines.first().is_some_and(|l| l.width() == 0) {
            lines.remove(0);
        }
        if todos {
            let list = crate::ui::todo_lines(
                self.board,
                self.config,
                self.width.saturating_sub(2),
                usize::from(self.height / 3),
                false,
            );
            if !list.is_empty() {
                lines.push(Line::default());
                lines.extend(list.into_iter().map(|l| {
                    let mut spans = vec![ratatui::text::Span::raw("  ")];
                    spans.extend(l.spans);
                    Line::from(spans)
                }));
            }
        }
        if !lines.is_empty() {
            lines.insert(0, Line::default());
        }
        lines
    }

    fn ctx(&self, transcript: &Transcript) -> Ctx<'_> {
        Ctx {
            config: self.config,
            human: self.human,
            indent: String::new(),
            width: self.width,
            hover: None,
            frame: self.frame,
            md: &self.media.md,
            figures: &self.media.figures,
            cards: &self.media.cards,
            diagrams: &self.media.diagrams,
            writing: transcript.writing().map(|e| e.id),
            level: transcript.level,
            screen_rows: self.height,
            replaced: transcript.replaced_shown(),
            sessions: &[],
            board: Some(self.board),
        }
    }
}

/// 卡片那一行指着的展示的文件路径：找不到那一条的（行没带指向）照这一轮最后一张卡片。
fn shown_path(transcript: &Transcript, target: Option<crate::ui::rows::Target>) -> Option<String> {
    let at = match target {
        Some(crate::ui::rows::Target::Entry(i)) => transcript.entries.get(i),
        _ => transcript.entries.iter().rev().find(|e| e.shown.is_some()),
    }?;
    at.shown.as_ref().map(|s| s.path.clone())
}
