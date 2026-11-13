//! 「文件修改」那一页（蓝图 `tui.md`「文件修改」第 2 条，发版前 check 9，2026-11-13 主人 1113 初验）：点侧边栏那一块的一行、
//! `/files` 开；整屏归它：左边看选中那个文件的差异（这个会话里改它的每一步照先后接起来，中间一道细线，同网页），右边完整的
//! 文件列表。`↑` `↓`（`j` `k`）换文件，`PgUp` `PgDn`、滚轮滚差异，点右边一行换过去，`Esc` 回会话（正文、输入框都照原样）。
//! 差异先照参数比的画，同时照每一步的调用编号问核心完整的（`view.detail`，带行号），回来了换上。

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use serde_json::{Value, json};

use super::App;
use crate::core::{Command, Update};
use crate::diff::Diff;
use crate::oobe::asker::TAG_BASE;

/// 问完整差异的请求编号从这里往下数，五百个轮一圈（配置合起来那一条用 `TAG_BASE - 1000` 往下的一千个）。
const DETAIL_TAG: u64 = TAG_BASE - 3000;
/// 轮一圈的个数。
const DETAIL_SPAN: u64 = 500;
/// 滚轮一格滚几行。
const WHEEL: usize = 3;

/// 这一页的状态。
#[derive(Debug, Default)]
pub struct FilesPage {
    /// 选中第几个文件（`changes::of` 的下标）。
    pub selected: usize,
    /// 差异滚到第几行。
    pub top: usize,
    /// 核心给的完整差异：调用编号到那一份；算不出差异的（`skipped`）不记，照参数比的画。
    pub details: HashMap<String, Rc<Diff>>,
    /// 问过的调用，不重问。
    asked: HashSet<String>,
    /// 问出去还没回来的：请求编号到调用编号。
    flying: HashMap<u64, String>,
    /// 下一个请求编号用第几个。
    sent: u64,
}

impl App {
    /// 开这一页，选上第 `index` 个文件；这个会话还没改过文件的说一句。
    pub(super) fn open_files(&mut self, index: usize) {
        let files = crate::changes::of(&self.talk.transcript, &self.config.timeline);
        if files.is_empty() {
            let text = self.config.text.files.none.clone();
            return self.hint(text, false);
        }
        let page = self.boxes.files_page.get_or_insert_with(FilesPage::default);
        page.selected = index.min(files.len() - 1);
        page.top = 0;
        self.screen.draw_now = true;
        self.ask_details();
    }

    /// 点了侧边栏「文件修改」的第 `row` 行（标题下面起）：开那一页选上这个文件；「还有 N 个」那一行选上第一个没露出来的。
    pub(super) fn files_clicked(&mut self, row: u16) {
        let index = self
            .screen
            .files
            .get(usize::from(row))
            .copied()
            .unwrap_or(self.config.layout.sidebar_files);
        self.open_files(index);
    }

    /// 这一页开着时的一个事件。
    pub(super) fn files_event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Release => {}
            Event::Key(key) => self.files_key(key.code),
            Event::Mouse(mouse) => {
                let at = ratatui::layout::Position::new(mouse.column, mouse.row);
                self.screen.pointer = Some(at);
                match mouse.kind {
                    MouseEventKind::ScrollDown => self.files_scroll(WHEEL, true),
                    MouseEventKind::ScrollUp => self.files_scroll(WHEEL, false),
                    MouseEventKind::Down(_) if self.screen.files_page.list.contains(at) => {
                        let row = usize::from(at.y - self.screen.files_page.list.y);
                        if let Some(index) = self.screen.files_page.rows.get(row).copied() {
                            self.files_select(index);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn files_key(&mut self, code: KeyCode) {
        let Some(page) = self.boxes.files_page.as_ref() else {
            return;
        };
        let selected = page.selected;
        let view = usize::from(self.screen.files_page.diff.height).max(1);
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.boxes.files_page = None;
                self.screen.draw_now = true;
            }
            KeyCode::Down | KeyCode::Char('j') => self.files_select(selected + 1),
            KeyCode::Up | KeyCode::Char('k') => self.files_select(selected.saturating_sub(1)),
            KeyCode::PageDown | KeyCode::Char(' ') => self.files_scroll(view, true),
            KeyCode::PageUp => self.files_scroll(view, false),
            KeyCode::Home => self.files_scroll(usize::MAX, false),
            KeyCode::End => self.files_scroll(usize::MAX, true),
            _ => {}
        }
    }

    /// 换到第 `index` 个文件（到头停）：差异从头看，问它的完整差异。
    fn files_select(&mut self, index: usize) {
        let count = crate::changes::of(&self.talk.transcript, &self.config.timeline).len();
        if let Some(page) = self.boxes.files_page.as_mut() {
            page.selected = index.min(count.saturating_sub(1));
            page.top = 0;
        }
        self.ask_details();
    }

    /// 差异往下（`down`）或往上滚几行；到底照画的时候限住（`ui/files_page.rs`）。
    fn files_scroll(&mut self, by: usize, down: bool) {
        let most = self.screen.files_page.most;
        if let Some(page) = self.boxes.files_page.as_mut() {
            page.top = if down {
                page.top.saturating_add(by).min(most)
            } else {
                page.top.saturating_sub(by)
            };
        }
    }

    /// 选中那个文件每一步的完整差异，没问过的问核心（`view.detail {session, call}`）。
    fn ask_details(&mut self) {
        let Some(session) = self.talk.transcript.session.clone() else {
            return;
        };
        let files = crate::changes::of(&self.talk.transcript, &self.config.timeline);
        let Some(page) = self.boxes.files_page.as_mut() else {
            return;
        };
        let Some(file) = files.get(page.selected) else {
            return;
        };
        for call in file.changes.iter().filter_map(|c| c.call.clone()) {
            if !page.asked.insert(call.clone()) {
                continue;
            }
            let tag = DETAIL_TAG - page.sent % DETAIL_SPAN;
            page.sent += 1;
            page.flying.insert(tag, call.clone());
            let params = json!({"session": session, "call": call});
            self.link.core.send(Command::Ask {
                tag,
                method: "view.detail",
                params,
            });
        }
    }

    /// 完整差异的回应归这里（交回 `true`）；这一页关了才回来的不要（下次开再问）。
    pub(super) fn files_page_update(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        let Some(page) = self.boxes.files_page.as_mut() else {
            return (DETAIL_TAG - DETAIL_SPAN < *tag) && (*tag <= DETAIL_TAG);
        };
        let Some(call) = page.flying.remove(tag) else {
            return false;
        };
        if let Ok(got) = result
            && let Some(diff) = detail_diff(got)
        {
            page.details.insert(call, Rc::new(diff));
            self.screen.draw_now = true;
        }
        true
    }
}

/// `view.detail` 回应里那一次调用改的文件的差异（编辑、写入只改一个文件，取第一个带差异的）；算不出的（`skipped`）是 `None`。
/// `diff` 是统一格式，照行给的数组、整段字都认。
pub fn detail_diff(got: &Value) -> Option<Diff> {
    let file = got["files"]
        .as_array()?
        .iter()
        .find(|f| !f["diff"].is_null())?;
    let lines: Vec<String> = match &file["diff"] {
        Value::Array(lines) => lines
            .iter()
            .filter_map(|l| l.as_str().map(str::to_string))
            .collect(),
        Value::String(text) => text.lines().map(str::to_string).collect(),
        _ => return None,
    };
    (!lines.is_empty()).then(|| crate::diff::unified::read(&lines))
}
