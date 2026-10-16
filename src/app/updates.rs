//! 收核心那边的消息（蓝图 `tui.md`「连核心」「正文」）：交给正文之前，界面这一头先办的几件——没发出去的撤回来、
//! 只弹提示的拒绝、撤销恢复时输入框里的那句、视口跟不跟、系统通知、被退回的排队消息放回输入框。

use super::App;
use crate::core::{Block, Command, Compaction, Push, Update};
use crate::transcript::Kind;

impl App {
    /// 收一条核心那边的消息。撤销成了、排队的消息被退回了，字放回输入框（`tui.md`「输入框」第 7、8 条）。
    pub fn core(&mut self, update: Update) {
        // 断开了：在途的请求不会回来了，等着的几处先复位（`lost.rs`）；下面各处照样收到这一条。
        if matches!(update, Update::Disconnected) {
            self.lost_requests();
            // 界面块跟着扩展一起停了：清掉，重连以后核心照记着的订阅重推（`widgets.rs`）。
            self.link.widgets.blocks.clear();
        }
        // 读配置合起来的那一条回来了：照原来的编号一处一处交回去（`config_asks.rs`）。
        if self.config_asks_update(&update) {
            return;
        }
        // 引导的回应归它；连上、断开也告诉它；头的配置读到了去看引导走过没有（`oobe.rs`）。
        if self.oobe_update(&update) {
            return;
        }
        // 终端自己的配置项（图标、时间线的开关）的回应归它；头的配置读到了也去读（`head_settings.rs`）。
        if self.head_settings_update(&update) {
            return;
        }
        // 换吉祥物的几个回应归它；头的配置读到了也去读（`mascot_pick.rs`）。
        if self.mascot_pick_update(&update) {
            return;
        }
        // 上面几处要读的配置合成一条，抢在人格、模型列表、给人看的字这些前头发（`config_asks.rs`）。
        self.flush_config_asks();
        // 人格头像的回应归它（`avatars.rs`）。
        if self.avatar_update(&update) {
            return;
        }
        // 配置页的回应归它；配置变了、断开、连上也告诉它（`settings.rs`）。
        if self.settings_update(&update) {
            return;
        }
        // 确认、提问归抽屉；工具有了结果的收起它的抽屉（`asking.rs`）。
        if self.asking_update(&update) {
            return;
        }
        // 命令的输出：记进任务表，不经正文（`output.rs`）。
        if let Update::Output {
            session,
            job,
            output,
            after,
        } = update
        {
            self.took_output(&session, &job, output, after);
            return;
        }
        // 都在冷却：向核心要一次模型资料，看最早几时恢复（「配置与模型」第 7 条）。
        if let Update::Push(Push::CallFailed(error)) = &update
            && error.class == "cooling"
        {
            self.link.core.send(Command::ListModels);
        }
        // 工具目录变了：照现在的语言重取给人看的字，新装的脚本、MCP、扩展工具的显示名跟着换（核心 9-8 七补）；重取软件包
        // 列表，包的图标跟着换（`packages.rs`）。
        if matches!(update, Update::ToolsChanged) {
            let code = self.config.language.code().to_string();
            self.link.core.send(Command::FetchHuman(code));
            self.ask_packages();
            return;
        }
        // 给人看的字到了：换上（包的图标是另外要的，留着），排好的行里有旧的工具名，扔掉重排（`human.rs`）。
        if let Update::Human(mut human) = update {
            human.set_icons(self.link.human.icons().clone());
            self.link.human = human;
            *self.media.row_cache.borrow_mut() = Default::default();
            return;
        }
        // 配置里的界面语言：照它换（`language.rs`）。
        // 连上核心都会读一次界面语言：顺手要一次模型资料，底栏照它写模型和思考强度（`effort.rs`）。
        // `/usage` 的三样：交给框（`usage.rs`）。
        if let Update::UsageRows { kind, rows } = update {
            self.usage_rows(kind, rows);
            return;
        }
        // 软件包：照终端那一项的状态目录放输入历史（`packages.rs`）。
        if let Update::Packages(list) = update {
            self.packages_arrived(&list);
            return;
        }
        // 人格：记下，该问的时候开人格框（`persona.rs`）。
        if let Update::Personas(list) = update {
            self.note_personas(&list);
            self.listed(super::new_session::Kind::Persona, list);
            return;
        }
        if let Update::Presets(list) = update {
            self.listed(super::new_session::Kind::Preset, list);
            return;
        }
        if let Update::HeadConfig(head) = update {
            // 连上、配置变了都会来：默认人格、默认预设照它，两样的列表顺手再要一次（新会话的框用）。核心照收到的先后答，大的
            // 排在后面（「连核心」第 9 条）：读配置（上面已经发了）、人格、预设、软件包，再是给人看的字（头一次）；底栏的模型
            // 资料等读到默认的聊天模型再要，只要它那一个（`effort.rs`）。
            self.picks_configured(head.persona.clone(), head.preset.clone());
            self.ask_packages();
            if !std::mem::replace(&mut self.link.human_asked, true) {
                let code = self.config.language.code().to_string();
                self.link.core.send(Command::FetchHuman(code));
            }
            self.note_head(&head.language, head.persona.as_deref());
            // 核心认的斜杠命令：连上、配置变了（装卸了包、写了模板的照样算）问一次。
            self.ask_catalog();
            self.link.currency = head.currency;
            self.language_from_config(&head.language);
            return;
        }
        // `/model` 的一行行：交给开着的框（`models.rs`）。
        // `@` 文件列表的回应：交给列表，照哪个词问的认（`mention/`）。
        if let Update::Files { word, result } = update {
            self.typing.mention.replied(&word, result.as_ref());
            return;
        }
        // 链接卡片、它的图回来了（`cards.rs`）。
        let mut held = Some(update);
        if self.card_update(&mut held) || self.diagram_update(&mut held) {
            return;
        }
        let Some(update) = held else {
            return;
        };
        // 换了模型：底栏照新的模型写思考强度，不等下一轮（2026-10-02 项目主人报）。
        if let Update::Configured(_) = &update {
            self.talk.transcript.update(update, &self.config.text);
            self.refresh_effort();
            return;
        }
        // `/effort` 的几级：交给开着的框（`effort.rs`）。
        if let Update::Efforts(list) = update {
            self.efforts_listed(list);
            return;
        }
        if let Update::Choices(all) = update {
            self.models_listed(all);
            return;
        }
        // 会话列表：交给开着的框（`switch.rs`）。
        if let Update::Sessions(all) = update {
            self.sessions_listed(all);
            return;
        }
        // 会话列表变了（核心 9-5）：改手上这一份，框开着的跟着变。
        if let Update::SessionChanged(change) = update {
            self.session_changed(change);
            return;
        }
        // 主会话不存在了：写一句，下一句话开新会话（`session.rs`，review B3）。
        if let Update::SessionGone(session) = update {
            self.session_gone(&session);
            return;
        }
        // 还没开会话时 `/workspace` 验完了（`workspace.rs`）。
        if let Update::DirChecked(result) = update {
            self.dir_checked(result);
            return;
        }
        // 按页读：订阅回应的三格、更早的一页照会话办（`pages.rs`）。
        let Some(update) = self.page_update(update) else {
            return;
        };
        // 照会话分：别的会话的交给停放着的那一份；任务的几种、别处来的话照任务表、正文先办（`sessions.rs`）。
        let Some(update) = self.route(update) else {
            return;
        };
        if let Update::Push(push) = &update
            && self.visible_push(push)
        {
            return;
        }
        // Ctrl+C 打断、要退回排着的：这一轮结束了撤掉它，撤掉了放回输入框（`takeback.rs`）。
        self.takeback_on(&update);
        // 没发出去：先撤掉先画上的那句、字放回输入框，接着照一般的拒绝办（`redo.rs`）。
        let update = match update {
            Update::Unsent { reason, message } => {
                self.take_back_unsent();
                Update::Refused { reason, message }
            }
            update => update,
        };
        // 切过去补发不成（会话删了、日志坏了）：照样换上来，下面弹原因（「会话列表」第 5 条）。
        if matches!(update, Update::Refused { .. }) {
            self.opened();
        }
        // 这几种拒绝只弹提示框，不写进正文（`tui.md`「正文」第 9 条）：认得的写自己的短话，命令清单里列着的弹核心的原话
        // （「长期目标」第 2 条）。
        if let Update::Refused {
            reason: Some(reason),
            message,
        } = &update
            && let Some(hint) = self.refusal_hint(reason, message)
        {
            self.hint(hint, false);
            return;
        }
        // 命令的回执弹提示，去掉句末的句号，和别的提示一样；`/clear` 不弹，正文里已有「上下文已清空」那一行（蓝图
        // 「斜杠命令」`/stop`，2026-10-07 项目主人：提示框里不要句号）。
        // 核心认的斜杠命令：清单到了接上；交给核心的那一行被拒弹原话、放回输入框（`catalog.rs`）。
        if let Update::Catalog(listed) = &update {
            self.catalog_arrived(listed);
            return;
        }
        // 界面块：整份、一块一块换；调方法被拒的弹原话（`widgets.rs`）。
        if self.widget_update(&update) {
            return;
        }
        if let Update::SlashRefused { message, .. } = &update {
            self.slash_refused(message);
            return;
        }
        if let Update::CommandRan { command, said } = &update {
            self.slash_ran();
            let said = said.trim_end().trim_end_matches(['。', '.']);
            // `/goal` 看进度的：原文长了截原文，在哪一步、第几轮留着（「长期目标」第 1 条）。
            let (text, tail) = match command.as_str() {
                "goal" => super::goal::split_progress(said),
                _ => (said, ""),
            };
            if command != "clear" && !said.is_empty() {
                self.hint_keeping(text.to_string(), tail.to_string(), false);
            }
            return;
        }
        if let Update::Renamed(title) = &update {
            let texts = &self.config.text.rename;
            let note = title.as_ref().map_or_else(
                || texts.removed.clone(),
                |title| texts.done.replace("{title}", title),
            );
            self.hint(note, false);
            return;
        }
        let undone = matches!(update, Update::Undone { restore: false, .. });
        if matches!(update, Update::Undone { restore: true, .. }) {
            self.typing.input.take_back();
        }
        // 限制了进行中那一段的高度就不放开视口（`tui.md`「正文」第 1 条、「时间线」第 20 条）。
        // 限制着的，一轮结束运行状态行收起时也按住不往下落。
        let release = crate::ui::release_on_fold(&self.config.timeline);
        if matches!(update, Update::Push(crate::core::Push::TurnEnded(_))) {
            if release {
                self.screen.view.settle();
            } else {
                self.screen.view.hold();
            }
        }
        // 撤销藏起了几轮：放开一次「只往下走」，上面的内容落下来补满，撤销那一行贴着底部（2026-10-02 项目主人定：
        // 原来停在上面、底下空着，点开以后又掉到下面）。
        if matches!(update, Update::Push(Push::Reverted(_))) {
            self.screen.view.settle();
        }
        // 她开始下一步：长正文替人停着的，回到最底下接着跟（`tui.md`「正文」第 1 条）。
        if let Update::Push(Push::BlockStart { block, .. }) = &update
            && *block != Block::Text
        {
            self.screen.view.resume();
        }
        // 系统通知、报给 herdr：在正文收它之前量这一轮用了多久（「系统通知」）。
        self.notify_core(&update);
        // 提前在后台压的：正文不写，弹「已触发上下文压缩」（「正文」第 9 条）。
        self.background_compaction(&update);
        // 清空了：像 Ctrl+L 一样清屏，「上下文已清空」在新的一屏顶上（「正文」第 9 条）。
        if matches!(update, Update::Push(Push::Compacted { clear: true })) {
            self.screen.view.clear();
        }
        let folds = self.talk.transcript.folds();
        // 开轮以前按过两下 `Esc`：正文收了这一轮的开头再打断（`takeback.rs`）。
        let stop = super::takeback::stops_on_start(self.talk.transcript.starting, &update);
        self.talk.transcript.update(update, &self.config.text);
        if stop {
            self.stop_started_turn();
        }
        // 在子会话里看：它那一行正在做什么、用了多少跟着更新。
        if let Some(child) = self.talk.viewing().map(str::to_string) {
            self.refresh_agent(&child);
        }
        // 一段刚收起（她开口、一轮结束）：放开一次视口，收起留下的空白由上面的行补满（`tui.md`「正文」第 1 条）。
        if release && self.talk.transcript.folds() > folds {
            self.screen.view.settle();
        }
        // 被退回的排队消息连同粘贴块放回输入框（`takeback.rs`）。
        let returned = self.talk.transcript.take_returned();
        self.put_returned(returned);
        if undone {
            let said = self
                .talk
                .transcript
                .entries
                .iter()
                .rev()
                .find(|e| e.kind == Kind::Undo);
            if let Some(said) = said.map(|e| e.text.clone()) {
                self.typing.input.put_back(&said);
            }
        }
    }

    /// 提前在后台压：开始的那一刻弹「已触发上下文压缩」；旧核心不推开始的，没停下来等就换上了的那一刻弹（「正文」第 9 条，
    /// 2026-10-10 项目主人：「异步压缩的时候是不需要有任何提示的，可以给一个通知说：已触发上下文压缩，但是文中不要有」）。
    fn background_compaction(&mut self, update: &Update) {
        let Update::Push(Push::Compaction(push)) = update else {
            return;
        };
        let say = match push {
            Compaction::Started => {
                self.talk.compaction_announced = true;
                true
            }
            Compaction::Done { prepared: true, .. } if !self.talk.transcript.compacting_shown() => {
                !std::mem::take(&mut self.talk.compaction_announced)
            }
            Compaction::Done { .. } | Compaction::Failed(_) => {
                self.talk.compaction_announced = false;
                false
            }
            _ => false,
        };
        if say {
            let text = self.config.text.compaction.triggered.clone();
            self.hint(text, true);
        }
    }

    /// 被拒时弹的提示（原因码 `reason`、核心的原话 `message`）：认得的原因码写自己的短话；命令清单里列着的照核心的原话，
    /// 去掉句末的句号，和别的提示一样；别的是 `None`，写进正文。
    fn refusal_hint(&self, reason: &str, message: &str) -> Option<String> {
        if let Some(hint) = self.config.text.refusal_hints.get(reason) {
            return Some(hint.clone());
        }
        let listed = self
            .config
            .commands
            .hinted_refusals
            .iter()
            .any(|r| r == reason);
        let said = message.trim_end().trim_end_matches(['。', '.']);
        (listed && !said.is_empty()).then(|| said.to_string())
    }
}
