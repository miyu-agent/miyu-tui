//! 她给人看的文件怎么开（蓝图 `tui.md`「展示」第 6–8 条）：点图、点卡片用程序开；实时收到的 HTML、PDF 当场开一次；
//! 还没读回来的等 `blob.get` 回来再开；都开不了的卡片写「没有能打开它的程序」和副本的路径，图弹一句。

use std::path::{Path, PathBuf};

use super::App;
use crate::core::Shown;
use crate::launch::{Env, Launched, launch};
use crate::shown_files::{SCHEME, ZOOM};

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
    pub(super) fn shown_arrived(&mut self, blob: &str) {
        let id = self
            .talk
            .transcript
            .entries
            .iter()
            .rev()
            .find(|e| e.shown.as_ref().is_some_and(|f| f.blob == blob))
            .map(|e| e.id);
        if let Some(id) = id {
            self.open_shown(id);
        }
    }

    /// 开第 `id` 条展示的文件；后来同一个路径又展示过的开最新的那一份（第 5 条）。还没读回来的记下，回来了再开。
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
        let saved = self
            .media
            .cards
            .borrow_mut()
            .file(&file.blob)
            .map(Path::to_path_buf);
        match saved {
            Some(saved) => self.launch_shown(id, &file, &saved),
            None => self.media.opening.entry(file.blob).or_default().push(id),
        }
    }

    /// 读回来了一个 blob：等着它的那几条现在开。
    pub(super) fn shown_saved(&mut self, blob: &str, path: Option<&Path>) {
        let Some(ids) = self.media.opening.remove(blob) else {
            return;
        };
        let Some(path) = path else {
            // 核心那边读不出这份内容（第 6 条，2026-10-17 项目主人定）。
            let note = self.config.text.shown.missing.clone();
            self.hint(note, false);
            return;
        };
        for id in ids {
            let file = self
                .talk
                .transcript
                .entries
                .iter()
                .find(|e| e.id == id)
                .and_then(|e| e.shown.clone());
            if let Some(file) = file {
                self.launch_shown(id, &file, path);
            }
        }
    }

    /// 拷成带文件名的副本、交给程序开；都开不了的卡片记下副本的路径（最后一行换成黄字），图弹一句（第 7 条）。
    fn launch_shown(&mut self, id: u64, file: &Shown, saved: &Path) {
        let copy = crate::shown_files::copy(&file.blob, &file.name, saved);
        // 副本拷不出来（读回来的那份没了、临时目录写不进去）：弹「已失效」（第 6 条，2026-10-17 项目主人定）。
        if copy.is_none() {
            let note = self.config.text.shown.expired.clone();
            self.hint(note, false);
            return;
        }
        let opened = copy
            .as_deref()
            .map(|copy| launch(copy, &file.media_type, &Env::current()));
        if matches!(opened, Some(Launched::Default | Launched::Browser)) {
            return;
        }
        let place: PathBuf = copy.unwrap_or_else(|| saved.to_path_buf());
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
