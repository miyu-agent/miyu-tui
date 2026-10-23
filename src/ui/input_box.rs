//! 输入框这一格：圆角框里画输入的字、提示符、暂存标记，编辑上一句时上边框写一句，上边框右边写子代理是谁、人起的标题；抽屉开着时框里画抽屉（蓝图 `tui.md`「输入框」、
//! 「确认和提问的抽屉」第 2 条）；新会话开头选人格、预设时框里画那个列表（「新会话：人格、工作区」第 1 条）。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use unicode_width::UnicodeWidthStr;

use super::{Areas, drawer};
use crate::app::{Screen, Seen};
use crate::config::Config;
use crate::core::Level;
use crate::drawer::Drawer;
use crate::input::{InputBox, VisualLine};
use crate::theme;

/// 画输入框以前先算好的几样（只读的那一份借不进来：画的时候输入框、抽屉要改自己滚到哪）。
pub(super) struct Facts {
    /// 权限级别：提示符的颜色。
    level: Level,
    /// 空着时写的提示。
    placeholder: String,
    /// 上边框右边写的：子代理是谁、人起的标题（`goal_tag.rs`）。
    tag: Option<String>,
    /// 焦点在输入框上：光标显示出来。
    typing: bool,
}

impl Facts {
    /// 照这一帧要读的算好。`home`：是首页。
    pub(super) fn of(seen: &Seen, home: bool) -> Self {
        Self {
            level: seen.talk.transcript.level,
            placeholder: placeholder(seen.config, home, seen.typing.tips.at()).to_string(),
            tag: super::goal_tag::border(
                seen.talk.viewing_title(),
                seen.talk
                    .transcript
                    .title
                    .as_deref()
                    .filter(|_| seen.talk.transcript.named),
                seen.config,
            ),
            // 焦点移到框下面（按钮、子代理状态行）、后台面板开着时，框里不放光标：打的字不进这里（2026-09-30 项目主人）。
            typing: seen.focus() == crate::focus::Focus::Input && seen.boxes.panel.is_none(),
        }
    }
}

/// 输入框：抽屉开着时框里画抽屉，别的时候画输入的字。画的时候能改的：输入框、开着的抽屉自己滚到哪，上一帧画面（抽屉
/// 每一行是第几项、光标）（蓝图 `app.md` 第三节）。
pub(super) fn draw_box(
    frame: &mut Frame,
    areas: Areas,
    (config, facts): (&Config, Facts),
    (input, pick): (
        &mut InputBox,
        Option<(super::panel::Chrome, Vec<Line<'static>>)>,
    ),
    drawer: Option<&mut Drawer>,
    screen: &mut Screen,
) {
    // 新会话开头选人格、预设（`start_pick.rs`）：框里画列表，不画输入的字、不放光标。
    if let Some((chrome, lines)) = pick.filter(|_| drawer.is_none()) {
        let set = crate::sheet::read(|s| s.frame.input.set());
        frame.render_widget(super::panel::block_with(chrome, set), areas.frame);
        frame.render_widget(Paragraph::new(lines), areas.text);
        screen.drawer_rows.clear();
        // 藏着的光标停在框里第一格（输入框出来时就在原地）：不挪的话停在上一帧的地方，一起来是左上角，选完一跳，开了
        // 拖尾的终端还画一道（2026-10-22 项目主人：「光标会在左上角，选择完人格和预设后又从左上跳到输入框」）。
        screen.caret.put(
            ratatui::layout::Position::new(areas.text.x, areas.text.y),
            false,
        );
        return;
    }
    if let Some(d) = drawer {
        frame.render_widget(
            Block::bordered()
                .border_set(crate::sheet::read(|s| s.frame.input.set()))
                .border_style(theme::dim()),
            areas.frame,
        );
        screen.drawer_rows = drawer::draw(
            frame,
            areas.text,
            d,
            &config.text.drawer,
            &config.layout.drawer_preview,
            config.layout.vim,
            &mut screen.caret,
        );
        return;
    }
    screen.drawer_rows.clear();
    let look = Look {
        level: facts.level,
        placeholder: &facts.placeholder,
        tag: facts.tag,
    };
    draw_input(frame, areas, input, config, look);
    // 焦点不在这里也停到插入点，只是不显示（蓝图「每一帧」）。
    if let Some(pos) = input.cursor_position() {
        screen.caret.put(pos, facts.typing);
    }
}

/// 框里框外要照的几样。
struct Look<'a> {
    /// 权限级别：提示符的颜色。
    level: Level,
    /// 空着时写的提示。
    placeholder: &'a str,
    /// 上边框右边写的：子代理是谁（「切进子会话」第 2 条）、人起的标题（「输入框」第 14 条）。
    tag: Option<String>,
}

fn draw_input(frame: &mut Frame, areas: Areas, input: &mut InputBox, config: &Config, look: Look) {
    let Look {
        level,
        placeholder,
        tag,
    } = look;
    let mut border = Block::bordered()
        .border_set(crate::sheet::read(|s| s.frame.input.set()))
        .border_style(theme::dim());
    // 编辑上一句：上边框写一句，框线的颜色（「输入框」第 13 条）。
    if input.editing() {
        border = border.title(Line::styled(config.text.editing.as_str(), theme::dim()));
    }
    // 上边框右边：切进了子会话写它是谁（「切进子会话」第 2 条，2026-09-30 项目主人照 Claude Code 定），不然写人起的标题
    // （「输入框」第 14 条）；长期目标写在框上面那一行（`goal_tag.rs`，2026-10-14 项目主人定）。
    if let Some(tag) = tag {
        border = border.title_top(Line::styled(tag, theme::dim()).right_aligned());
    }
    frame.render_widget(border, areas.frame);
    let scroll = input.place(areas.text);
    let editor = &input.editor;
    let blocks = editor.blocks();
    // 空着：提示从光标后面空一格写起，光标停在空格上，不压着提示的第一个字（「输入框」第 9 条）。
    let body = if editor.is_empty() {
        vec![Line::from(vec![
            Span::raw(PLACEHOLDER_GAP),
            Span::styled(placeholder, theme::dim()),
        ])]
    } else {
        let lines = input.lines(areas.text.width);
        lines
            .iter()
            .skip(scroll)
            .take(usize::from(areas.text.height))
            .map(|l| {
                let hover = input.hovered_attachment();
                styled_line(editor.text(), *l, editor.selection(), &blocks, hover)
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(body), areas.text);
    // 暂存着东西：第一行最右边暗色写一个标记；和第一行的字挤不下就不写。
    if input.stashed() {
        let mark = config.text.stashed.as_str();
        let first = if editor.is_empty() {
            PLACEHOLDER_GAP.width() + placeholder.width()
        } else {
            input
                .lines(areas.text.width)
                .get(scroll)
                .map_or(0, |l| editor.text()[l.start..l.end].width())
        };
        // 标记和字之间至少空一列。
        if first + mark.width() < usize::from(areas.text.width) {
            let row = Rect::new(areas.text.x, areas.text.y, areas.text.width, 1);
            frame.render_widget(
                Paragraph::new(Line::styled(mark, theme::dim()).right_aligned()),
                row,
            );
        }
    }
    // 提示符画在第一行文字的左边，住在 pad_left 那几列里，颜色跟着权限级别；放不下就不画。
    // 暂存着东西换成这一套图标的 `stash`（`tui.md`「输入框」第 6 条、「图标」）。
    let prompt = if input.stashed() {
        config.icons.stash.as_str()
    } else {
        config.layout.prompt.as_str()
    };
    let prompt_style = theme::level(level);
    let width = u16::try_from(prompt.width()).unwrap_or(u16::MAX);
    // 框里左边那几列（边线和字之间）放得下提示符才画；紧凑版面正好放下（`tui.md`「输入框」第 10 条）。
    let room = areas.text.x.saturating_sub(areas.frame.x + 1);
    if scroll == 0 && room >= width {
        let at = Rect::new(areas.text.x - width, areas.text.y, width, 1);
        frame.render_widget(Paragraph::new(Line::styled(prompt, prompt_style)), at);
    }
}

/// 空着时提示前面空的那一格：光标停在它上面。
const PLACEHOLDER_GAP: &str = " ";

/// 输入框空着时写的提示（`tui.md`「输入框」第 9 条）：首页固定写怎么切权限级别，别处照轮到的那一条。
pub(super) fn placeholder(config: &Config, home: bool, tip: usize) -> &str {
    if home {
        return &config.text.home_placeholder;
    }
    config.text.tips.get(tip).map_or("", String::as_str)
}

/// 一行字，选中的那一段反色；鼠标悬停着的附件块（`hover`）加下划线（「输入框」第 12 条）。
fn styled_line<'a>(
    text: &'a str,
    line: VisualLine,
    selection: Option<(usize, usize)>,
    blocks: &[(usize, usize)],
    hover: Option<(usize, usize)>,
) -> Line<'a> {
    use ratatui::text::Span;
    // 这一行里样子变的地方：选区、粘贴块的两头。
    let mut cuts = vec![line.start, line.end];
    for &(s, e) in selection.iter().chain(blocks) {
        cuts.extend([s, e].map(|at| at.clamp(line.start, line.end)));
    }
    cuts.sort_unstable();
    cuts.dedup();
    let inside =
        |range: Option<&(usize, usize)>, at: usize| range.is_some_and(|&(s, e)| s <= at && at < e);
    let spans = cuts
        .windows(2)
        .map(|w| {
            let style = if inside(selection.as_ref(), w[0]) {
                theme::selected()
            } else if let Some(b) = blocks.iter().find(|b| inside(Some(b), w[0])) {
                let chip = theme::chip();
                if hover == Some(*b) {
                    chip.add_modifier(ratatui::style::Modifier::UNDERLINED)
                } else {
                    chip
                }
            } else {
                ratatui::style::Style::new()
            };
            Span::styled(&text[w[0]..w[1]], style)
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::styled_line;
    use crate::input::VisualLine;
    use crate::theme;

    #[test]
    fn a_paste_block_is_a_chip_and_the_selection_wins() {
        // 输入框里的粘贴块品红（`tui.md`「输入框」第 11 条）；选中的照旧反色。
        let text = "看[已粘贴 12 行]吗";
        let line = VisualLine {
            start: 0,
            end: text.len(),
        };
        let block = ("看".len(), text.len() - "吗".len());
        let got = styled_line(text, line, None, &[block], None);
        let label = got
            .spans
            .iter()
            .find(|s| s.content == "[已粘贴 12 行]")
            .unwrap();
        assert_eq!(label.style, theme::chip(), "品红字、暗紫底");
        assert!(theme::chip().bg.is_some());
        let all: String = got.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(all, text);
        let chosen = styled_line(text, line, Some((0, text.len())), &[block], None);
        assert!(
            chosen
                .spans
                .iter()
                .all(|s| s.content.is_empty() || s.style == theme::selected())
        );
    }
}
