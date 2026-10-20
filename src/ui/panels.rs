//! 输入框上面那个框里画什么（蓝图「后台命令、子代理和侧边栏」第 3 条、`/help`、`/model`、`/sessions`、`/effort`、
//! `/language`、`/usage`）：照开着的是哪一个排成行，交回框（标题、提示）、行、每行是第几项（点中用）。

use std::time::Instant;

use ratatui::text::Line;

use super::panel::Chrome;
use super::{
    background, effort_list, help, languages, model_list, persona_list, session_list, spin_frame,
    themes, usage,
};
use crate::app::Seen;

/// 开着的框排成的行，最多 `max` 行。
pub(super) fn content(
    seen: &Seen,
    width: u16,
    now: Instant,
    max: usize,
) -> (Chrome, Vec<Line<'static>>, Vec<Option<usize>>) {
    match seen.boxes.panel {
        Some(crate::app::Panel::Help { scroll }) => {
            let (chrome, lines) = help::lines(seen.config, width, scroll, max);
            let map = vec![None; lines.len()];
            (chrome, lines, map)
        }
        Some(crate::app::Panel::Models) => match &seen.boxes.model_list {
            Some(list) => model_list::lines(
                list,
                seen.talk.transcript.current_ref().as_deref(),
                seen.config,
                width,
                max,
            ),
            None => Default::default(),
        },
        Some(crate::app::Panel::Sessions) => match &seen.boxes.session_list {
            Some(list) => session_list::lines(
                list,
                session_list::Marks {
                    current: seen.talk.main_session().as_deref(),
                    asking: &seen.talk.drawers.held_owners(),
                },
                seen.cwd,
                seen.config,
                width,
                max,
                spin_frame(seen),
            ),
            None => Default::default(),
        },
        Some(crate::app::Panel::Effort { selected }) => effort_list::lines(
            seen.boxes.efforts.as_ref(),
            selected,
            seen.config,
            width,
            max,
        ),
        Some(crate::app::Panel::Language { selected }) => languages::lines(
            seen.config,
            &seen.link.system_language,
            selected,
            width,
            max,
        ),
        Some(crate::app::Panel::Theme { selected }) => {
            let open = seen.boxes.theme_box.as_ref();
            let options = open.map_or(&[][..], |b| b.options.as_slice());
            let before = open.map_or("", |b| b.before.as_str());
            themes::lines(seen.config, (options, before), selected, width, max)
        }
        Some(crate::app::Panel::Usage { tab, scroll, money }) => {
            let (chrome, lines) = usage::lines(seen, (tab, scroll, money), width, max);
            let map = vec![None; lines.len()];
            (chrome, lines, map)
        }
        Some(crate::app::Panel::Pick { kind, selected }) => {
            let (list, default) = seen.pick_list(kind);
            // 开着会话的是中途换（「新会话：人格、工作区」第 2a 条）：现在用的那一个；还没开会话、已经选过的是选的那一个
            // （2026-10-20 项目主人报：空会话选了人格、预设，再打开列表没有圆点）。
            let t = &seen.talk.transcript;
            let switching = t.session.is_some();
            let current = if switching {
                Some(match kind {
                    crate::app::new_session::Kind::Persona => t.persona.as_deref().unwrap_or(""),
                    crate::app::new_session::Kind::Preset => t.preset.as_deref().unwrap_or(""),
                })
            } else {
                match kind {
                    crate::app::new_session::Kind::Persona => seen.talk.persona.chosen.as_deref(),
                    crate::app::new_session::Kind::Preset => seen.talk.preset.chosen.as_deref(),
                }
            };
            persona_list::lines(
                list,
                default,
                (current, switching),
                &seen.config.layout.current_mark,
                kind,
                selected,
                &seen.config.text.persona_box,
                width,
                max,
            )
        }
        Some(crate::app::Panel::Changed { kind, yes, .. }) => {
            let words = &seen.config.text.settings.in_use;
            persona_list::changed_lines(kind, yes, words, width, max)
        }
        _ => background::lines(
            seen.boxes.panel,
            &seen.talk.board,
            seen.config,
            width,
            now,
            max,
        ),
    }
}
