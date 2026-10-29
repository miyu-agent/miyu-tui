//! 按键：列表开着时先归列表，别的给输入框；两下 `Esc`、翻正文、清屏、斜杠命令的执行（`tui.md`「按键」）。

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::App;
use crate::commands::{Line, Run, Spec};
use crate::core::Command;
use crate::input::{Action, Draft};
use crate::transcript::Starting;

impl App {
    /// 按了要按两下的 `Esc`：在回答时打断（排着队的接着发），没在回答、前台在整理时叫停整理。
    /// 上一下还在时限里就做，不然只提示再按一次。
    pub(super) fn esc(&mut self) {
        let window = Duration::from_millis(self.config.layout.esc_window_ms);
        // 发出去了、核心还没开轮的也算在回答：记下，轮一开就打断（`takeback.rs`）。
        let starting = self.talk.transcript.starting != Starting::No;
        let running = self.talk.transcript.running.is_some() || starting;
        match self.typing.esc_at.take() {
            Some(at) if at.elapsed() <= window => {
                self.typing.notice = None;
                if running && self.switching() {
                    return;
                }
                if starting {
                    self.talk.transcript.starting = Starting::Stop;
                } else if !running && self.dreaming_here() {
                    // 她在做梦：叫停整理（`dream.rs`，「整理记忆」第 7 条）。
                    self.dream_stop();
                } else if running {
                    // 她还没开始做事、又没有排着的：打断以后撤掉这一轮，那一句放回输入框（「按键」`Esc`，2026-10-07
                    // 项目主人定；和 Ctrl+C 退回排着的走同一条路，`takeback.rs`）。
                    let queued = self
                        .talk
                        .transcript
                        .entries
                        .iter()
                        .any(|e| e.queued && !e.hidden);
                    if !queued && let Some(back) = self.talk.transcript.untouched() {
                        let session = self.talk.transcript.session.clone();
                        self.talk.takeback =
                            Some(super::takeback::Takeback::Interrupting(session, back));
                    }
                    self.link.core.send(Command::Interrupt { send: true });
                }
            }
            _ => {
                self.typing.esc_at = Some(Instant::now());
                let text = &self.config.text;
                let hint = if running {
                    &text.esc_hint
                } else {
                    // 没在回答：前台在整理才会到这里，叫停整理（「整理记忆」第 7 条）。
                    &text.dream.esc_hint
                };
                self.hint(hint.clone(), false);
            }
        }
    }

    /// 换下一套图标，只管这一次启动（`tui.md`「图标」第 4 条）。排好的时间线照图标那一套的名字重排。
    pub(super) fn next_icons(&mut self) {
        let sets = &self.config.icon_sets;
        let at = sets
            .iter()
            .position(|s| s.name == self.config.icons.name)
            .unwrap_or(0);
        if let Some(next) = sets.get((at + 1) % sets.len().max(1)).cloned() {
            let text = self.config.text.icons_changed.replace("{name}", &next.name);
            self.config.icons = next;
            self.hint(text, true);
        }
    }

    /// 按键：列表开着时，上下、Tab、Enter、Esc 归列表，别的照旧给输入框。
    pub(super) fn key(&mut self, key: KeyEvent) -> Action {
        // Ctrl+V（Alt+V 同）：读剪贴板，照粘贴处理；列表、抽屉开着时也一样（`tui.md`「按键」）。
        if is_paste(&key) {
            self.paste_clipboard();
            return Action::None;
        }
        // 输入历史列表开着时，按键先归它（`tui.md`「输入历史列表」）。
        if self.typing.history.open {
            self.history_key(key);
            return Action::None;
        }
        // 确认和提问的抽屉开着时按键先归它（`tui.md`「确认和提问的抽屉」第 4 条）。
        if self.drawer_key(key) {
            return Action::None;
        }
        // 新会话开头那个换掉输入框的人格、预设框（列表读到以前写「正在加载…」也算）：没有输入框，`Ctrl+C`、`Ctrl+D` 直接
        // 退出，`Ctrl+Z` 照常挂起（2026-10-23 项目主人：「这个界面我期望能 ctrl+C/Ctrl+D 退出 tui」）。
        if self.start_box_shown() && leaves(&key) && key.code != KeyCode::Char('z') {
            self.quit = true;
            return Action::None;
        }
        // 新会话开头的框开着、在读时打的字进输入框（「新会话：人格、工作区」第 1c 条）。
        if self.start_box_typed(key) {
            return Action::None;
        }
        // 新会话开头还没选人格、预设，列表还在读：输入框的位置写「正在加载…」，别的按键哪都不进，只认退出、挂起
        // （「新会话：人格、工作区」第 1 条，2026-10-21 项目主人：会话开得慢时输入框能打几个字）。
        if self.start_loading() && !leaves(&key) {
            return Action::None;
        }
        // Tab、Shift+Tab（终端报成 BackTab）轮换权限级别；命令列表、`@` 文件列表开着时 Tab 归列表（`tui.md`「按键」）。
        let menu_open = self.menu_matches().is_some() || self.mention_found().is_some();
        // 整屏看输出、后台面板、后台按钮先拿按键（`tui.md`「后台命令、子代理和侧边栏」）。
        if self.jobs_key(key, menu_open) {
            return Action::None;
        }
        if cycles_level(&key, menu_open) {
            self.cycle_level();
            return Action::None;
        }
        // 正文里有选区：Ctrl+C、Ctrl+Shift+C 复制它，Esc 取消它；都先于输入框（`tui.md`「按键」）。
        let ctrl = key
            .modifiers
            .contains(ratatui::crossterm::event::KeyModifiers::CONTROL);
        // 侧边栏有选区：Ctrl+C 复制它，Esc 取消它（`tui.md`「后台命令、子代理和侧边栏」第 7 条）。
        if self.screen.side_select.active() {
            if ctrl && matches!(key.code, KeyCode::Char('c' | 'C')) {
                let text = self.screen.side_select.text();
                self.copy(&text);
                return Action::None;
            }
            if key.code == KeyCode::Esc {
                self.screen.side_select.clear();
                return Action::None;
            }
        }
        if self.screen.view.select.is_some() {
            if ctrl && matches!(key.code, KeyCode::Char('c' | 'C')) {
                let text = self.screen.view.selected_text();
                self.copy(&text);
                return Action::None;
            }
            if key.code == KeyCode::Esc {
                self.screen.view.select = None;
                return Action::None;
            }
        }
        // Ctrl+L：清屏，把视口顶空，往回滚内容还在；回答进行中不清（新的字马上又冒出来）。
        if ctrl && key.code == KeyCode::Char('l') {
            if self.talk.transcript.running.is_some() {
                self.hint(self.config.text.no_clear_running.clone(), false);
            } else {
                self.screen.view.clear();
            }
            return Action::None;
        }
        // PgUp、PgDn：翻正文。
        if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
            self.screen.view.page(key.code == KeyCode::PageDown);
            return Action::None;
        }
        // Ctrl+R：调出输入历史列表；恢复撤销只用 /restore（`tui.md`「按键」）。
        if ctrl && key.code == KeyCode::Char('r') {
            if self.typing.input.sent().is_empty() {
                self.hint(self.config.text.history.none.clone(), false);
            } else {
                self.typing.history.open();
            }
            return Action::None;
        }
        // Ctrl+End：回到正文最底下，跟着最新的（`tui.md`「按键」，2026-10-02 项目主人要）。
        if ctrl && key.code == KeyCode::End {
            self.screen.view.follow();
            return Action::None;
        }
        // Ctrl+G：用编辑器写输入框里的话（`compose.rs`，`tui.md`「按键」）。
        if ctrl && key.code == KeyCode::Char('g') {
            self.compose();
            return Action::None;
        }
        // Ctrl+Z：挂起到后台；主循环照 `suspend` 还原终端、发信号（`tui.md`「按键」）。Windows 没有作业控制。
        if ctrl && key.code == KeyCode::Char('z') {
            self.suspend = cfg!(unix);
            return Action::None;
        }
        // Ctrl+Enter：输入框有字先发（在回答时排进队），在回答的当场打断、排着的马上发（`tui.md`「按键」，
        // 2026-10-02 项目主人要；不提示）。没在回答时同 Enter。
        if ctrl && key.code == KeyCode::Enter && self.menu_matches().is_none() {
            let running = self.talk.transcript.running.is_some();
            let plain = KeyEvent::new(
                KeyCode::Enter,
                ratatui::crossterm::event::KeyModifiers::NONE,
            );
            let action = self.typing.input.key(plain);
            if running {
                if let Action::Submit(text) = action {
                    self.submit(text);
                }
                // 没有排着的（输入框也是空的）：什么都不做，不当打断用。
                if self.talk.transcript.entries.iter().any(|e| e.queued) && !self.switching() {
                    self.link.core.send(Command::Interrupt { send: true });
                }
                return Action::None;
            }
            return action;
        }
        // `@` 文件列表开着：选、进目录、收成块、关（「`@` 文件列表」第 5 条）；别的键照常打字。
        if let Some(found) = self.mention_found()
            && let Some(action) = self.mention_key(key, &found)
        {
            return action;
        }
        let Some(matches) = self.menu_matches() else {
            return self.typing.input.key(key);
        };
        let selected = matches.get(self.typing.menu.selected).cloned();
        match (key.code, selected) {
            // `↑`（`Ctrl+K`、vim 的 `k` 换成的也是它）只在列表里往上挪，不翻输入历史（2026-10-20 项目主人：「出命令列表就应该
            // 不允许调出输入历史了」；原来 10-17 定真的 `↑` 交给输入框翻历史，`Ctrl+K` 换成的 `↑` 也跟着翻了过去）。
            (KeyCode::Up, _) => self.typing.menu.step(false, matches.len()),
            (KeyCode::Down, _) => self.typing.menu.step(true, matches.len()),
            (KeyCode::Tab, Some(spec)) => {
                self.typing.input.editor.take();
                self.typing.input.paste(&format!("/{}", spec.name));
            }
            (KeyCode::Enter, Some(spec)) => {
                self.typing.input.editor.take();
                self.run(&spec, None);
            }
            (KeyCode::Esc, _) => self.typing.menu.dismiss(self.typing.input.editor.text()),
            _ => return self.typing.input.key(key),
        }
        Action::None
    }

    /// 回车发出去的字：是命令就执行；像命令、终端又没有这个命令的交给核心，被拒的弹核心的原话、字放回输入框（`tui.md`
    /// 「斜杠命令列表」第 4 条）；别的发给她。
    pub(super) fn submit(&mut self, draft: Draft) {
        let text = draft.text.clone();
        // 在编辑上一句：打了命令就不编辑了；打错了命令、发不出去的，接着编辑（「输入框」第 13 条）。
        let edit = self.typing.input.take_edit();
        match self.config.commands.read(&text) {
            Line::Command(spec, words) => {
                let (spec, words) = (spec.clone(), words.map(str::to_string));
                self.run(&spec, words.as_deref());
                return;
            }
            // 终端不认的名字：交给核心，由它说为什么不行（核心可能有，只是这个会话用不了，`data.why` 说得清；协议
            // `command.catalog` 第 2 条）；被拒的弹它的原话、字放回输入框（`catalog.rs`）。在编辑上一句的接着编辑。
            Line::Unknown => {
                if let Some(original) = edit {
                    self.typing.input.resume_edit(original);
                }
                self.typing.input.remember(draft.clone());
                self.slash(&text);
                return;
            }
            Line::Talk => {}
        }
        // 连不上核心：发不出去，字留在输入框里（「连核心」第 8 条）。
        if !self.reachable() {
            self.typing.input.editor.set_draft(draft);
            if let Some(original) = edit {
                self.typing.input.resume_edit(original);
            }
            return;
        }
        if let Some(original) = edit {
            self.submit_edit(original, draft);
            return;
        }
        self.leave_gone();
        self.screen.view.follow();
        // 附了以后才换的模型：收不了的附件改成文件块写路径（第 12 条「照模型收」）。
        let mut draft = draft;
        self.demote_for_send(&mut draft);
        // 输入框里的粘贴块发出去换回原文；输入历史和正文里照输入框的样子（`tui.md`「输入框」第 11 条）。附件（图）的
        // 块照留 `[图片 1]`，文件跟着发（第 12 条）。
        let full = draft.expand();
        let chips = super::paste::chips(&draft);
        let files = draft.attachments();
        self.talk.unsent = Some(super::Unsent {
            draft: Some(draft.clone()),
            original: None,
            turn: None,
        });
        self.typing.input.remember(draft);
        self.typing.input.forget_cleared();
        // 她在做梦：说话把她叫醒，整理转到后台（`dream.rs`）。
        self.dream_wake_up();
        self.talk.transcript.user(text, chips);
        self.awaiting_turn();
        self.link.core.send(Command::Send {
            text: full,
            files,
            cwd: None,
        });
    }

    /// 执行一条命令；`words` 是名字后面的字（能带参数的命令才有，蓝图「斜杠命令列表」第 5 条）。
    pub(super) fn run(&mut self, spec: &Spec, words: Option<&str>) {
        // 命令也记进输入历史：从列表里回车、点的，和整条打出来回车的一样，带参数的记整条（`tui.md`「按键」↑、↓）。
        let line = match words {
            Some(words) => format!("/{} {words}", spec.name),
            None => format!("/{}", spec.name),
        };
        self.typing.input.remember(Draft::plain(&line));
        self.screen.view.follow();
        match spec.run {
            Run::Revert
            | Run::Unrevert
            | Run::Compact
            | Run::Clear
            | Run::Stop
            | Run::Redo
            | Run::Edit
            | Run::Recap
            | Run::Rename
            | Run::Goal
                if !self.reachable() =>
            {
                self.typing.input.editor.set_draft(Draft::plain(&line));
            }
            Run::Revert
            | Run::Unrevert
            | Run::Compact
            | Run::Clear
            | Run::Stop
            | Run::Redo
            | Run::Edit
            | Run::Recap
            | Run::Rename
                if self.not_opened() =>
            {
                self.nothing_yet(spec.run);
            }
            Run::Redo => self.redo(),
            Run::Edit => self.edit_last(),
            Run::Clear => self.link.core.send(Command::Run("/clear".into())),
            Run::Stop => self.link.core.send(Command::Run("/stop".into())),
            Run::Recap => {
                self.link.core.send(Command::Recap);
                let note = self.config.text.recap.working.clone();
                self.hint(note, false);
            }
            Run::Rename => self.rename(words),
            Run::Goal => self.goal_command(words),
            Run::Workspace => self.workspace_command(words.unwrap_or_default()),
            Run::Sessions => self.open_sessions(),
            Run::Jump => self.open_jump(),
            Run::Model => self.open_models(),
            Run::Persona => self.pick_command(super::new_session::Kind::Persona),
            Run::Preset => self.pick_command(super::new_session::Kind::Preset),
            Run::Config => self.open_settings(false),
            Run::Connect => self.open_connect(),
            Run::Effort => self.open_effort(),
            Run::Copy => self.copy_reply(),
            Run::Usage => self.open_usage(),
            Run::Status => self.open_status(),
            Run::ToolList => self.open_tool_list(),
            Run::Context => self.open_context(),
            Run::New => self.new_session(),
            Run::Revert => self.undo(),
            Run::Unrevert => self.link.core.send(Command::Unrevert),
            Run::Compact => self.compact(words),
            Run::Theme => self.open_themes(),
            Run::Icons => self.next_icons(),
            Run::Images => self.images_command(words),
            Run::Quit => self.quit = true,
            Run::Help => self.open_help(),
            Run::Language => self.open_languages(),
            // 扩展的整页带的命令：当场开那一页，不找核心（不为开一页先开会话，「界面块」第 10 条）。
            Run::Core if spec.opens.is_some() => {
                if let Some(page) = spec.opens.clone() {
                    self.open_widget_page(page);
                }
            }
            // 核心认的命令（模板、扩展声明的）：整行交给核心（`catalog.rs`）。
            Run::Core => self.slash(&line),
        }
    }

    /// `/copy`：复制她上一轮的回答，照拖选以后 Ctrl+C 那一套；还没有回答的弹一句（蓝图「斜杠命令」`/copy`）。
    fn copy_reply(&mut self) {
        match self.talk.transcript.last_reply() {
            Some(text) => self.copy(&text),
            None => {
                let note = self.config.text.nothing_to_copy.clone();
                self.hint(note, false);
            }
        }
    }

    /// 输入历史列表开着时的按键：`↑`、`Ctrl+R` 往更早的走，`↓` 往更新的走，打字进搜索，
    /// `Enter` 挑中放进输入框，`Tab` 展开全文，`Esc` 关掉（`tui.md`「输入历史列表」第 2–4、7 条）。
    fn history_key(&mut self, key: KeyEvent) {
        use ratatui::crossterm::event::KeyModifiers;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let plain = !ctrl && !key.modifiers.contains(KeyModifiers::ALT);
        let count = self.typing.history.matches(&self.list_items().0).len();
        match key.code {
            KeyCode::Up => self.typing.history.older(count),
            KeyCode::Char('r') if ctrl => self.typing.history.older(count),
            KeyCode::Down => self.typing.history.newer(),
            KeyCode::Enter => self.pick_history(),
            KeyCode::Tab => {
                let selected = self.typing.history.selected;
                let (list, _) = self.list_items();
                let found = self.typing.history.matches(&list);
                if let Some(at) = found.get(selected).map(|s| s.at) {
                    self.typing.history.toggle_full(at);
                }
            }
            KeyCode::Esc => self.typing.history.close(),
            KeyCode::Backspace => self.typing.history.backspace(),
            KeyCode::Char(c) if plain => self.typing.history.type_text(&c.to_string()),
            _ => {}
        }
    }

    /// 挑中选中的那一条：放进输入框，关掉列表；`/jump` 的跳过去（「跳到说过的话」第 3 条）。
    pub(super) fn pick_history(&mut self) {
        let (text, id) = {
            let (list, ids) = self.list_items();
            let found = self.typing.history.matches(&list);
            let picked = found.get(self.typing.history.selected).copied();
            // 列表是照先后排的一份，挑中的是其中哪一句：照位置找回它的条目编号。
            let id = picked
                .and_then(|p| list.iter().position(|s| std::ptr::eq(s, p)))
                .and_then(|i| ids.get(i).copied());
            (picked.map(|sent| sent.draft.text.clone()), id)
        };
        let jump = self.typing.history.jump;
        self.typing.history.close();
        match (jump, id, text) {
            (true, Some(id), _) => self.jump_to(id),
            (false, _, Some(text)) => self.typing.input.pick(&text),
            _ => {}
        }
    }

    /// 前台压缩（`/compact`，人格框、预设框里改了正在用的答「是」也走这里）：旧核心的压缩进度不说手动、自动，自己发的照
    /// 手动画进度条（「正文」第 9 条）。
    pub(super) fn compact(&mut self, words: Option<&str>) {
        self.talk.transcript.manual_compaction = true;
        self.link
            .core
            .send(Command::Compact(words.map(str::to_string)));
    }
}

/// 这一下是不是读剪贴板：`Ctrl+V`（认得 kitty 键盘协议的终端里按着 Shift 报成大写的 V，不算：那是终端自己的粘贴），
/// 或者 `Alt+V`（Windows Terminal 自己占着 `Ctrl+V`，剪贴板里只有图时什么都不送进来，2026-10-26 虚拟机里实测；同 Claude Code）。
pub(crate) fn is_paste(key: &KeyEvent) -> bool {
    use ratatui::crossterm::event::KeyModifiers;
    key.code == KeyCode::Char('v')
        && (key.modifiers == KeyModifiers::CONTROL || key.modifiers == KeyModifiers::ALT)
}

/// 这一下是不是轮换权限级别：`Shift+Tab` 什么时候都是；`Tab` 在命令列表没开时是（开着时补全命令）。
fn cycles_level(key: &KeyEvent, menu_open: bool) -> bool {
    key.code == KeyCode::BackTab
        || (key.code == KeyCode::Tab && key.modifiers.is_empty() && !menu_open)
}

/// 列表还在读时也认的键：`Ctrl+C`、`Ctrl+D`（退出）、`Ctrl+Z`（挂起）。
fn leaves(key: &KeyEvent) -> bool {
    key.modifiers
        .contains(ratatui::crossterm::event::KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c' | 'd' | 'z'))
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{cycles_level, is_paste};

    #[test]
    fn tab_and_shift_tab_cycle_the_level_unless_the_menu_wants_tab() {
        let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let back = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert!(cycles_level(&tab, false), "Tab 轮换");
        assert!(cycles_level(&back, false), "Shift+Tab 留着");
        assert!(!cycles_level(&tab, true), "命令列表开着：Tab 补全命令");
        assert!(cycles_level(&back, true));
    }

    #[test]
    fn ctrl_v_reads_the_clipboard_but_ctrl_shift_v_is_the_terminals() {
        let key = |code, m| KeyEvent::new(code, m);
        assert!(is_paste(&key(KeyCode::Char('v'), KeyModifiers::CONTROL)));
        // 按着 Shift 的是终端自己的粘贴（走括号粘贴），不读剪贴板。
        assert!(!is_paste(&key(
            KeyCode::Char('V'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        )));
        assert!(!is_paste(&key(KeyCode::Char('v'), KeyModifiers::NONE)));
        assert!(is_paste(&key(KeyCode::Char('v'), KeyModifiers::ALT)));
    }
}
