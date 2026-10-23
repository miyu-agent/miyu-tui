//! 画一帧：上面是正文，下面是圆角的输入框，框下面一行是用量。
//!
//! 版式照网页的聊天输入框：框居中、跟着窗口变宽，正文和框一样宽；框里只有文字，
//! 框下面一行左边是模式和模型，右边是临时的状态和用量。

mod agents;
mod avatar_view;
mod background;
mod body;
mod bottom_button;
mod compaction_rows;
mod diff_rows;
mod done_row;
mod drawer;
mod dream_rows;
mod dreamt_rows;
pub use drawer::PreviewLook;
pub mod effort_list;
mod figure_rows;
mod floating;
mod footer;
mod foreign_rows;
mod goal_tag;
pub mod help;
pub mod hint_keys;
mod history;
mod home;
mod input_box;
mod job_rows;
pub mod languages;
mod layout;
mod link_card;
mod margins;
mod mascot_view;
mod md_cache;
mod media_rows;
pub mod model_list;
mod oobe;
pub mod shown_rows;
pub use oobe::idle_look as oobe_idle_look;
mod panel;
mod panels;
pub mod persona_list;
pub mod session_list;
pub mod settings;
mod sidebar;
pub mod themes;
pub mod usage;
pub mod widget;

pub use history::{index_at as history_index_at, lines as history_lines};
pub use menu::{index_at as menu_index_at, rows as menu_rows};
mod mention;
mod menu;
pub mod row_cache;
pub mod rows;
mod start_pick;
mod status;
pub mod status_box;
mod timeline;
mod undo_rows;
pub use timeline::release_on_fold;
mod user_rows;
pub mod wide;

#[cfg(test)]
mod rulers;
#[cfg(test)]
mod rulers_live;
#[cfg(test)]
mod test_support;

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::App;

use layout::text_width;
pub use layout::{Areas, areas};
pub(super) use layout::{above, agents_area, box_width, list_text};

/// 画一帧。顺手把各块的位置记进 `app`，鼠标事件要用。
pub fn draw(frame: &mut Frame, app: &mut App) {
    app.screen.caret.begin();
    // 界面块能点的地方每一帧重记（`widget/place.rs`）。
    app.screen.widget_spots.clear();
    // 第一次打开的引导开着：整屏归它（蓝图「第一次打开的引导」第 1 条）。
    if let Some(oobe) = app.boxes.oobe.as_mut() {
        let mut parts = oobe::Parts {
            config: &app.config,
            media: &app.media,
            oobe,
            screen: &mut app.screen,
            mascot: &mut app.mascot,
        };
        oobe::draw(frame, &mut parts);
        return;
    }
    // 配置页开着：整屏归它，对话不画（图片也不画），关了照原样画回来（蓝图「配置页」第 1 条）。
    if let Some(page) = app.boxes.settings.as_mut() {
        settings::draw(frame, page, &app.config, &mut app.screen.caret);
        return;
    }
    // 扩展的整页开着：整屏归它（「界面块」第 2 条）。
    if let Some(view) = app.boxes.widget_page.clone() {
        let (seen, screen, _) = app.parts();
        widget::page(frame, &view, &seen, screen);
        return;
    }
    // 还在问 `ui.welcomed`：整屏空着，免得进引导前闪一下首页（「第一次打开的引导」第 1 条）。
    if app.seen().welcome_pending(std::time::Instant::now()) {
        return;
    }
    // 不在首页、够宽时右边分出侧边栏，别的都画在主列里（`tui.md`「后台命令、子代理和侧边栏」第 7 条）。
    let home = app.talk.blank();
    let (main, sidebar) = if home {
        let area = frame.area();
        (area, Rect::new(area.right(), area.y, 0, area.height))
    } else {
        sidebar::split(frame.area(), &app.config.layout)
    };
    // 要改状态的先做完：换提示、两个列表（几个列表占同一个地方，开哪个照 `app/frame.rs`）。
    let crate::app::Lists { mentions, matches } = app.before_frame();
    let running = app.talk.transcript.running.is_some();
    let queued: Vec<String> = app
        .talk
        .transcript
        .queued()
        .map(|e| e.text.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let count = u16::try_from(queued.len()).unwrap_or(u16::MAX);
    let agent_rows = agents::height(app.talk.listed_agents().len(), app.config.layout.agent_rows);
    // 没有侧边栏（窄屏、首页）时，待办常驻在输入框上面（第 4 条）；有侧边栏时在侧边栏里。
    let todo_lines = if sidebar.width == 0 {
        // 首页的框收窄了：照首页框里的字宽排，不然长的一项被裁掉、没有「…」。
        let width = if home {
            home::text_width(main.width, &app.config.layout)
        } else {
            text_width(main.width, &app.config.layout)
        };
        sidebar::todo_lines(
            &app.talk.board,
            &app.config,
            width,
            app.config.layout.todo_rows,
            app.talk.todo_full,
        )
    } else {
        Vec::new()
    };
    let todo_rows = u16::try_from(todo_lines.len()).unwrap_or(u16::MAX);
    // 首页的界面块照首页框里的字宽排好，要几行交给 `place` 定位（蓝图「界面块」第 4 条）。
    let home_blocks = if home {
        widget::home(
            &app.seen(),
            home::text_width(main.width, &app.config.layout),
        )
    } else {
        Vec::new()
    };
    let widget_rows = widget::home_rows(&home_blocks);
    // 框里占几行：抽屉开着时是抽屉的行数，连边框最高半屏；带文字画的放得下整张为止，屏幕顶上留几行
    // （`tui.md`「确认和提问的抽屉」第 2 条）。
    let texts = &app.config.text.drawer;
    let open_drawer = app.talk.drawers.current.as_ref();
    let half = (main.height / 2).saturating_sub(2).max(3);
    let drawer_max = match open_drawer {
        Some(d) if d.has_preview() => main
            .height
            .saturating_sub(app.config.layout.drawer_keep_rows + 4)
            .max(half),
        _ => half,
    };
    // 新会话开头选人格、预设：画在输入框的位置（`start_pick.rs`），占它排出来的行数。不问人格预设的（`tui.skip_pick`）
    // 等默认的读到以前照旧画输入框，不画「正在加载…」那个框（2026-10-23 项目主人：打开时一瞬间看到选择框，像闪了一下）；
    // 默认的用不了、真要选时框照常开。
    let skip = app.config.layout.skip_pick
        && !matches!(app.boxes.panel, Some(crate::app::Panel::Pick { .. }));
    let start = app.seen().start_pick().filter(|_| !skip);
    let box_rows = |w: u16| match (open_drawer, start) {
        (Some(d), _) => drawer::rows(d, texts, &app.config.layout.drawer_preview, w, drawer_max),
        (None, Some(kind)) => {
            let rows = start_pick::content(&app.seen(), kind, w, usize::from(half))
                .1
                .len();
            u16::try_from(rows).unwrap_or(half)
        }
        (None, None) => app.typing.input.rows(w),
    };
    // 抽屉这一页的预览比输入框宽：抽屉跟着放宽（最多整栏），选项往左让（2026-10-23 项目主人照 Claude Code，`drawer/preview.rs`）。
    let widened = open_drawer.and_then(|d| {
        let base = &app.config.layout;
        let chrome = layout::box_width(main.width, base) - layout::text_width(main.width, base);
        let most = main.width.saturating_sub(chrome);
        let want = drawer::preview_wanted(d, texts, &base.drawer_preview, most)?;
        if want <= layout::text_width(main.width, base) {
            return None;
        }
        let mut wide = base.clone();
        let box_w = u32::from(want.min(most) + chrome);
        wide.width_percent = u16::try_from((box_w * 100).div_ceil(u32::from(main.width.max(1))))
            .unwrap_or(100)
            .min(100);
        Some(wide)
    });
    let layout_used = widened.as_ref().unwrap_or(&app.config.layout);
    // 空会话是首页：整组上下居中，吉祥物在中间（`tui.md`「空会话的首页」）。列表占几行交给 `place` 定位。
    let place = |menu_rows: u16| {
        if home {
            let look = &app.config.mascot;
            // 首页的吉祥物能关（「后台命令、子代理和侧边栏」第 7 条）；人格有头像的照样留这一块（「空会话的首页」第 10 条）。
            let mascot = if avatar_view::slot_shown(&app.seen(), crate::app::Place::Home) {
                (look.cols, look.rows)
            } else {
                (0, 0)
            };
            home::areas(
                main,
                &box_rows,
                layout_used,
                mascot,
                menu_rows,
                agent_rows,
                todo_rows,
                widget_rows,
            )
        } else {
            areas(
                main,
                &box_rows,
                layout_used,
                menu_rows,
                running,
                count,
                agent_rows,
                todo_rows,
            )
        }
    };
    // 输入框上面的列表最多能占几行（连框）：放不下的由各列表自己瘦身（「窗口小的时候」第 1 条）。框里的几条照这一帧
    // 量出来的宽度排：首页的输入框窄一些（「斜杠命令列表」第 3 条）。
    let probe = place(u16::MAX);
    let room = probe.menu.height;
    let inner = panel::room(room);
    let now = std::time::Instant::now();
    let width = probe.menu_text.width;
    // 历史列表排成的行：占几行、画什么都照它（`history.rs` 的 `lines`）。
    let (history_chrome, history_rows) = if app.typing.history.open {
        let (list, _) = app.list_items();
        let found = app.typing.history.matches(&list);
        history::lines(
            &app.typing.history,
            &found,
            width,
            &app.config,
            std::time::SystemTime::now(),
            usize::from(inner),
        )
    } else {
        Default::default()
    };
    // 后台面板排成的行（第 3 条）；帮助框（`/help`）这些框一样放在这个位置（`panels.rs`）。由命令打开的框里上下各空一行
    // （`panel::PAD`），放不下的不空；打字时弹的列表不空。
    // 新会话开头的人格框、预设框画在输入框里，不占上面这一块。
    let panel_open = app.boxes.panel.is_some() && start.is_none();
    let pad = panel_open && inner > panel::PAD;
    let panel_room = if pad { inner - panel::PAD } else { inner };
    let (panel_chrome, panel_lines, panel_rows) =
        panels::content(&app.seen(), width, now, usize::from(panel_room));
    let menu_shown = menu::rows(app.config.layout.menu_rows, inner);
    // 框里的几条，放得下框的加上上下两条边（`tui.md`「斜杠命令列表」第 3 条）。
    let items = if panel_open {
        panel_lines.len() + if pad { usize::from(panel::PAD) } else { 0 }
    } else if app.typing.history.open {
        history_rows.len()
    } else if let Some(found) = &mentions {
        found.items.len().clamp(1, menu_shown)
    } else {
        matches.as_ref().map_or(0, |m| m.len().min(menu_shown))
    };
    let menu_rows = panel::height(u16::try_from(items).unwrap_or(u16::MAX), room);
    let mut areas = place(menu_rows);
    if pad {
        areas.menu_text = panel::padded(areas.menu_text);
    }
    areas.sidebar = sidebar;
    // 正文的滚动条贴在这一栏最右边（`body_view.rs` 的 `bar_x`）。
    app.screen.view.bar_x = Some(main.right().saturating_sub(1));
    let whole = frame.area();
    app.screen.pixel.size = (whole.width, whole.height);
    app.screen.areas = areas;
    app.screen.panel_rows = panel_rows;
    use crate::frame_log::section;
    if home {
        // 先画输入框（吉祥物照输入光标转头），再画吉祥物；开着列表时吉祥物已经让到列表上面。
        section("input", || {
            let facts = input_box::Facts::of(&app.seen(), home);
            let pick = start.map(|kind| {
                let max = usize::from(areas.text.height);
                start_pick::content(&app.seen(), kind, areas.text.width, max)
            });
            let drawer = app.talk.drawers.current.as_mut();
            let parts = (&app.config, facts);
            input_box::draw_box(
                frame,
                areas,
                parts,
                (&mut app.typing.input, pick),
                drawer,
                &mut app.screen,
            );
        });
        section("home", || {
            let (seen, screen, mascot) = app.parts();
            home::draw(frame, areas, &seen, screen, mascot);
            let pointer = screen.pointer;
            let spots = &mut screen.widget_spots;
            widget::draw_home(frame, areas.widgets, &home_blocks, &seen, pointer, spots);
        });
    } else {
        section("body", || {
            let (seen, screen, _) = app.parts();
            body::draw(frame, areas, &seen, screen);
        });
        section("input", || {
            let facts = input_box::Facts::of(&app.seen(), home);
            let pick = start.map(|kind| {
                let max = usize::from(areas.text.height);
                start_pick::content(&app.seen(), kind, areas.text.width, max)
            });
            let drawer = app.talk.drawers.current.as_mut();
            let parts = (&app.config, facts);
            input_box::draw_box(
                frame,
                areas,
                parts,
                (&mut app.typing.input, pick),
                drawer,
                &mut app.screen,
            );
        });
    }
    // 待办、运行状态行、排队的消息先画：输入框上面的框是覆盖层，开着时盖在它们上面（「斜杠命令列表」第 1 条）。
    frame.render_widget(ratatui::widgets::Paragraph::new(todo_lines), areas.todo);
    section("status", || {
        let (seen, _, mascot) = app.parts();
        status::draw(frame, areas.pulse, &seen, mascot);
    });
    // 指针停在哪一条排队的话上：那一行后面写「点击撤回」（2026-10-22 项目主人定点那条撤回）。
    let hovered = app
        .screen
        .pointer
        .filter(|p| areas.queued.contains(*p))
        .map(|p| p.y - areas.queued.y);
    let look = status::Queued {
        mark: &app.config.layout.queued_mark,
        hint: &app.config.text.queued_withdraw,
        hovered,
    };
    status::queued(frame, areas.queued, &queued, &look);
    // 长期目标的进度：框上面那一行右对齐，看着子会话时不写（「长期目标」第 5 条，2026-10-14 项目主人定）；列表、面板开着时
    // 盖在它上面（下面才画）。
    let goal = app
        .talk
        .viewing_title()
        .is_none()
        .then(|| goal_tag::goal(app.talk.transcript.goal.as_ref(), &app.config))
        .flatten()
        .filter(|_| !home);
    if let Some(goal) = goal {
        goal_tag::draw(frame, areas.goal, &goal);
    }
    if let Some(matches) = &matches {
        let rows = menu::rows(app.config.layout.menu_rows, areas.menu_text.height);
        let (chrome, lines) = menu::lines(
            matches,
            app.typing.menu.selected,
            app.typing.menu.pinned,
            rows,
            areas.menu_text.width,
            &app.config.text.menu,
        );
        menu::draw(frame, areas.menu, areas.menu_text, chrome, lines);
    }
    if let Some(found) = &mentions {
        let rows = menu::rows(app.config.layout.menu_rows, areas.menu_text.height);
        let (chrome, lines) = mention::lines(
            found,
            app.typing.mention.selected,
            app.typing.mention.pinned,
            rows,
            areas.menu_text.width,
            &app.config.text.mention,
        );
        menu::draw(frame, areas.menu, areas.menu_text, chrome, lines);
    }
    if app.typing.history.open {
        history::draw(
            frame,
            areas.menu,
            areas.menu_text,
            history_chrome,
            history_rows,
        );
    }
    background::draw(
        frame,
        areas.menu,
        areas.menu_text,
        panel_chrome,
        panel_lines,
    );
    let (button, status_spots) = section("footer", || {
        footer::draw(frame, areas.footer, &app.seen(), &app.screen)
    });
    app.screen.areas.button = button;
    app.screen.widget_spots.extend(status_spots);
    // 不在首页才有正文可翻。
    app.screen.areas.bottom = if home {
        Rect::default()
    } else {
        bottom_button::draw(frame, &app.seen(), &app.screen)
    };
    section("agents", || {
        agents::draw(frame, areas.agents, &app.seen(), &app.screen)
    });
    section("sidebar", || {
        let (seen, screen, mascot) = app.parts();
        sidebar::draw(frame, sidebar, &seen, screen, mascot);
    });
    // 提示最后画，浮在正文上面；底边紧贴输入框，在回答时紧贴运行状态行（`tui.md`「提示」）；开着框时贴着框的上边。
    let toast_bottom = if running {
        areas.pulse.y.saturating_sub(1)
    } else {
        areas.body.bottom()
    };
    let toast_bottom = if areas.menu.height > 0 {
        toast_bottom.min(areas.menu.y)
    } else {
        toast_bottom
    };
    status::toast(
        frame,
        areas.text.x,
        toast_bottom,
        areas.text.width,
        &app.seen(),
    );
}

#[cfg(test)]
mod tests;

/// 转圈转到第几帧：照界面起来以后过了多久，一帧 `timeline.spinner_ms`。
fn spin_frame(seen: &crate::app::Seen) -> usize {
    let spinner_ms = seen.config.timeline.spinner_ms.max(1);
    usize::try_from(seen.started.elapsed().as_millis() / u128::from(spinner_ms)).unwrap_or(0)
}
