//! 刚启动时的 [`App`]：照配置摆好各块、读回输入历史、起输入法那一头（从 `mod.rs` 分出来）。

use super::*;

impl App {
    /// 刚启动时的样子：输入框空着，正文空着，核心在连。
    pub fn new(config: Config, core: Core, human: Human, figures: Figures) -> Self {
        // 启动时的配置照系统语言读的（自动）：记下它，选回自动时用。
        let system_language = config.language.clone();
        let md_keep = config.layout.markdown_cache;
        let ime = crate::ime::Ime::start(&config.layout.ime, |name| std::env::var(name).ok());
        // 主题先照出厂的设置挑（跟随终端，终端底色还没问的当深色）；连上读到 `tui.theme*`、问到终端的底色再换（`theme.rs`）。
        let picked = crate::theme::choice::pick(
            &crate::theme::choice::Choice::default(),
            None,
            &config.themes,
            None,
        );
        let theme = picked.map_or_else(String::new, |p| {
            crate::theme::set(p.palette);
            crate::theme::set_ground(p.ground);
            p.name
        });
        let config_tips = config.text.tips.len();
        let layout = &config.layout;
        let mut input = InputBox::new(
            layout.max_rows,
            Duration::from_millis(layout.double_click_ms),
        );
        input.set_paste_rule(PasteRule {
            lines: layout.paste_fold_lines,
            chars: layout.paste_fold_chars,
            label: config.text.paste_label.clone(),
        });
        input.set_attach_rule(paste::attach_rule(&config));
        // 输入历史连上以后照 `package.list` 放进终端这个包的状态目录（`app/packages.rs`）；这里先记着 9-1 以前的老位置
        // （数据根的 `state/tui/history.jsonl`），拿不到状态目录的照旧用它。单元测试不记：数据根是人在用的。
        let old_history =
            miyu_client::places::DataRoot::locate(&miyu_client::places::Env::current())
                .ok()
                .filter(|_| !cfg!(test))
                .map(|root| root.state().join("tui").join("history.jsonl"));
        // 提示音、暂存的截图放机器缓存目录（「系统通知」第 5 条、「输入框」第 12 条）；找不到的不响、贴不了图。
        let cache = miyu_client::places::cache_root(&miyu_client::places::Env::current()).ok();
        let sounds = cache.as_ref().map(|root| root.join("tui").join("sounds"));
        let staging = cache
            .as_ref()
            .map(|root| crate::clipboard::Staging::new(&crate::clipboard::staging_root(root)));
        let mut notifier = Notifier::new(
            config.notify.clone(),
            config.text.notify.clone(),
            |name| std::env::var(name).ok(),
            sounds,
        );
        let mention = crate::mention::Mention::new(config.mention.clone());
        // 界面一开就报空闲：herdr 侧栏上马上看得到（「系统通知」第 6 条）。
        notifier.state(crate::notify::State::Idle);
        Self {
            config,
            notifier,
            mascot: super::mascot::MascotState::new(),
            screen: super::screen::Screen::new(theme),
            talk: super::talk::Talk {
                default_level: crate::core::Level::Workspace,
                level_picked: false,
                transcript: Transcript::default(),
                board: Board::default(),
                parked: Default::default(),
                home: None,
                left: Vec::new(),
                opening: None,
                sessions_seen: None,
                unsent: None,
                takeback: None,
                asks: Default::default(),
                drawers: Drawers::default(),
                todo_full: false,
                compaction_announced: false,
                workspace_pending: None,
                persona: super::new_session::Pick::from_env("MIYU_TUI_PERSONA"),
                preset: super::new_session::Pick::from_env("MIYU_TUI_PRESET"),
            },
            link: super::link::Link {
                core,
                abilities: crate::core::Abilities::default(),
                human,
                currency: "USD".to_string(),
                default_chat: None,
                system_language,
                human_asked: false,
                config_asks: Default::default(),
                look: Default::default(),
            },
            typing: super::typing::Typing {
                input,
                menu: Menu::default(),
                mention,
                history: History::default(),
                history_placed: false,
                old_history,
                focus: Focus::Input,
                esc_at: None,
                composing: None,
                staging,
                editor: crate::editor::command(|name| std::env::var(name).ok()),
                edit: None,
                ime,
                tips: Tips::new(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(1, |d| d.as_nanos() as u64 >> 3),
                    config_tips,
                ),
                notice: None,
                slash_draft: None,
            },
            boxes: Default::default(),
            media: super::media::Media::new(md_keep, figures),
            cwd: std::env::current_dir()
                .map(|d| crate::local::home_short(&d.display().to_string()))
                .unwrap_or_default(),
            quit: false,
            suspend: false,
        }
    }
}
