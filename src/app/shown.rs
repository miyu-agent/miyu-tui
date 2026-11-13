//! 她给人看的文件怎么开（蓝图 `tui.md`「展示」第 6–8 条）：点图、点卡片用程序开；实时收到的 HTML、PDF 当场开一次；
//! 照 `shown.path` 直接开（核心 SW-1，内容不进核心）；都开不了的卡片写「没有能打开它的程序」和路径，图弹一句。

use std::path::{Path, PathBuf};

use super::App;
use crate::core::Shown;
use crate::launch::{Env, Launched, launch};
use crate::shown_files::{Found, SCHEME, ZOOM};

impl App {
    /// 点了挂着 [`SCHEME`] 的那一格：开它；不是的交回 `false`。
    pub(super) fn open_shown_link(&mut self, url: &str) -> bool {
        // mermaid 的大图：同一个退路，开不了的弹一句（「图片、公式和 mermaid 图」第 4 条）。
        if let Some(file) = url.strip_prefix(ZOOM) {
            let file = Path::new(file);
            if launch(file, "image/svg+xml", &Env::current()) == Launched::Nothing {
                let words = &self.config.text.shown;
                let note = words
                    .no_program
                    .replace("{path}", &file.display().to_string());
                self.hint(note, false);
            }
            return true;
        }
        let Some(id) = url.strip_prefix(SCHEME).and_then(|n| n.parse().ok()) else {
            return false;
        };
        self.open_shown(id);
        true
    }

    /// 实时收到一次展示（正文已经收下）：HTML、PDF 当场开一次（第 8 条）。补发、翻历史的不走这里。
    pub(super) fn shown_arrived(&mut self, path: &str) {
        let id = self
            .talk
            .transcript
            .entries
            .iter()
            .rev()
            .find(|e| e.shown.as_ref().is_some_and(|f| f.path == path))
            .map(|e| e.id);
        if let Some(id) = id {
            self.open_shown(id);
        }
    }

    /// 开第 `id` 条展示的文件；后来同一个路径又展示过的开最新的那一份（第 5 条）。照 `path` 直接开（核心 SW-1：内容不进核心，
    /// 终端和核心同一台机器）；改过、没了的弹「文件已变」「文件已不在」。
    fn open_shown(&mut self, id: u64) {
        let entries = &self.talk.transcript.entries;
        let Some(path) = entries
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.shown.as_ref())
            .map(|f| f.path.clone())
        else {
            return;
        };
        let Some((id, file)) = entries.iter().rev().filter(|e| !e.hidden).find_map(|e| {
            e.shown
                .as_ref()
                .filter(|f| f.path == path)
                .map(|f| (e.id, f.clone()))
        }) else {
            return;
        };
        let words = &self.config.text.shown;
        match crate::shown_files::find(&file) {
            Found::Here(at) => self.launch_shown(id, &file, &at),
            Found::Changed => {
                let note = words.changed.clone();
                self.hint(note, false);
            }
            Found::Gone => {
                let note = words.gone.clone();
                self.hint(note, false);
            }
        }
    }

    /// 交给程序开；都开不了的卡片记下路径（最后一行换成黄字），图弹一句（第 7 条）。
    fn launch_shown(&mut self, id: u64, file: &Shown, at: &Path) {
        let opened = launch(at, &file.media_type, &Env::current());
        if matches!(opened, Launched::Default | Launched::Browser) {
            return;
        }
        let place: PathBuf = at.to_path_buf();
        let words = &self.config.text.shown;
        if file.image() {
            let note = words
                .no_program
                .replace("{path}", &place.display().to_string());
            self.hint(note, false);
            return;
        }
        if let Some(entry) = self.talk.transcript.entries.iter_mut().find(|e| e.id == id) {
            entry.unopened = Some(place.display().to_string());
        }
    }
}
