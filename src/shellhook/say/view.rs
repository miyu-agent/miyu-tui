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
    board: crate::jobs::Board,
}

impl Media {
    /// 照配置备好。
    pub fn new(config: &Config) -> Self {
        Self {
            md: RefCell::new(MdCache::new(config.layout.markdown_cache)),
            figures: RefCell::new(Figures::start(None, &config.figures, None, |_| true)),
            cards: RefCell::default(),
            diagrams: RefCell::default(),
            board: crate::jobs::Board::default(),
        }
    }
}

/// 照这几样排。
pub struct View<'a> {
    pub config: &'a Config,
    pub human: &'a Human,
    pub media: &'a Media,
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

    /// 排好的行只要画出来的样子。
    pub fn lines(&self, transcript: &Transcript, ours: Option<u64>) -> Vec<Line<'static>> {
        self.rows(transcript, ours)
            .into_iter()
            .map(|r| r.line)
            .collect()
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
            board: Some(&self.media.board),
        }
    }
}
