//! 扩展的整页（蓝图「界面块」第 2、3、9 条）：照配置页的版面居中、宽高八成；最上面一行面包屑 `● <软件名> — <标题>`，下面是
//! 内容，滚着看；最下面一条细线、一行按键提示。还没推来的写「正在读…」，程序没在跑的写「<软件名> 没有运行」。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::place::{Kind, Spot};
use super::{Look, Mode, content};
use crate::app::{Screen, Seen};
use crate::theme;
use crate::widgets::{Action, PageView, Place};

/// 上一帧整页画成什么样：一共几行、露几行、能点的在第几行。键盘照它选、照它滚。
#[derive(Debug, Default)]
pub struct PageFrame {
    pub rows: usize,
    pub view: usize,
    pub actions: Vec<(usize, Action)>,
}

/// 画整页。
pub fn draw(frame: &mut Frame, view: &PageView, seen: &Seen, screen: &mut Screen) {
    let area = frame.area();
    let stage = crate::ui::settings::stage(area, &seen.config.layout.settings);
    let texts = &seen.config.text.widget;
    let target = &view.target;
    let block = seen
        .link
        .widgets
        .find(&target.package, &target.widget)
        .filter(|b| b.place == Place::Page);
    let source = block.map_or(target.package.as_str(), |b| b.source.as_str());
    let title = block
        .and_then(|b| b.title.as_deref())
        .unwrap_or(&target.widget);
    let crumb = Line::from(vec![
        Span::styled(
            format!("{} ", crate::sheet::mark(|m| &m.on)),
            theme::accent(),
        ),
        Span::styled(source.to_string(), theme::dim()),
        Span::styled(" — ", theme::dim()),
        Span::styled(
            title.to_string(),
            theme::accent().add_modifier(Modifier::BOLD),
        ),
    ]);
    frame
        .buffer_mut()
        .set_line(stage.x, stage.y, &crumb, stage.width);
    if stage.height < 5 {
        return;
    }
    let body = Rect::new(stage.x, stage.y + 2, stage.width, stage.height - 5);
    let look = Look {
        width: body.width,
        mode: Mode::Page,
        marks: &seen.config.layout.widgets.marks,
        bar: &seen.config.layout.bar,
        texts,
    };
    let drawn = match block.and_then(|b| b.content.as_deref()) {
        Some(segments) => content(segments, &look, None),
        None => {
            let running = block.is_none_or(|b| b.running);
            let line = if running {
                texts.loading.clone()
            } else {
                texts.not_running.replace("{source}", source)
            };
            super::Drawn {
                lines: vec![Line::styled(line, theme::dim())],
                ..super::Drawn::default()
            }
        }
    };
    let height = usize::from(body.height);
    let scroll = view.scroll.min(drawn.lines.len().saturating_sub(height));
    let shown: Vec<Line> = drawn
        .lines
        .iter()
        .skip(scroll)
        .take(height)
        .cloned()
        .collect();
    frame.render_widget(Paragraph::new(shown), body);
    let mut actions = Vec::new();
    for hit in &drawn.hits {
        let index = actions.len();
        actions.push((hit.row, hit.action.clone()));
        let Some(row) = hit.row.checked_sub(scroll).filter(|r| *r < height) else {
            continue;
        };
        let rect = Rect::new(
            body.x + hit.from,
            body.y + u16::try_from(row).unwrap_or(0),
            hit.to - hit.from,
            1,
        )
        .intersection(body);
        let pointed = screen.pointer.is_some_and(|p| rect.contains(p));
        if pointed || index == view.selected {
            frame.buffer_mut().set_style(rect, theme::hover());
        }
        screen.widget_spots.push(Spot {
            rect,
            package: target.package.clone(),
            id: target.widget.clone(),
            what: Kind::Act(hit.action.clone()),
        });
    }
    screen.widget_page = PageFrame {
        rows: drawn.lines.len(),
        view: height,
        actions,
    };
    let rule = crate::sheet::mark(|m| &m.rule).repeat(usize::from(stage.width));
    let buf = frame.buffer_mut();
    buf.set_string(stage.x, stage.bottom() - 2, &rule, theme::faint());
    let hints = crate::ui::floating::hint_line(&texts.page_keys);
    buf.set_line(stage.x, stage.bottom() - 1, &hints, stage.width);
}
