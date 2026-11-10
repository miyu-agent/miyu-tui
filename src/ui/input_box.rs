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
    /// 查到 `stt`：右边画麦克风，照它在哪一步、转圈转到第几帧（设计 20 第七节）。
    mic: Option<(crate::voice::Mic, usize)>,
}

impl Facts {
    /// 照这一帧要读的算好。`home`：是首页。
    pub(super) fn of(seen: &Seen, home: bool) -> Self {
        Self {
            level: seen.talk.transcript.level,
            placeholder: if seen.typing.input.shell() {
                seen.config.text.shell.placeholder.clone()
            } else if seen.talk.archived() {
                seen.config.text.archived_placeholder.clone()
            } else if seen.talk.transcript.read_only {
                // 日志中间坏了、只能看的会话（核心 YC-6，`app/unread.rs`）：写怎么新开会话
                seen.config.text.unread.read_only_placeholder.clone()
            } else {
                placeholder(seen.config, home, seen.typing.tips.at()).to_string()
            },
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
            mic: seen.link.voice.stt().then(|| {
                let spinner_ms = u128::from(seen.config.timeline.spinner_ms.max(1));
                let spin = seen.started.elapsed().as_millis() / spinner_ms;
                (seen.link.voice.mic, usize::try_from(spin).unwrap_or(0))
            }),
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
        // 往下滑着的：下边那条线滑出去了，不画（「确认和提问的抽屉」第 4a 条）。
        let borders = if areas.slid > 0 {
            ratatui::widgets::Borders::TOP
                | ratatui::widgets::Borders::LEFT
                | ratatui::widgets::Borders::RIGHT
        } else {
            ratatui::widgets::Borders::ALL
        };
        frame.render_widget(
            Block::bordered()
                .borders(borders)
                .border_set(crate::sheet::read(|s| s.frame.input.set()))
                .border_style(theme::dim()),
            areas.frame,
        );
        screen.drawer_rows = drawer::draw(
            frame,
            (areas.text, areas.slid.saturating_sub(1)),
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
    if let Some(mic) = facts.mic {
        screen.areas.mic = draw_mic(frame, areas, mic, config);
    }
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
    // 跑命令的模式：框线品红（「人自己跑命令 `!`」第 1 条，照 Claude Code 的 bash 模式）。
    let shell = input.shell();
    let mut border = Block::bordered()
        .border_set(crate::sheet::read(|s| s.frame.input.set()))
        .border_style(if shell { theme::picked() } else { theme::dim() });
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
    let prompt = if shell {
        SHELL_PROMPT
    } else if input.stashed() {
        config.icons.stash.as_str()
    } else {
        config.layout.prompt.as_str()
    };
    let prompt_style = if shell {
        theme::picked()
    } else {
        theme::level(level)
    };
    let width = u16::try_from(prompt.width()).unwrap_or(u16::MAX);
    // 框里左边那几列（边线和字之间）放得下提示符才画；紧凑版面正好放下（`tui.md`「输入框」第 10 条）。
    let room = areas.text.x.saturating_sub(areas.frame.x + 1);
    if scroll == 0 && room >= width {
        let at = Rect::new(areas.text.x - width, areas.text.y, width, 1);
        frame.render_widget(Paragraph::new(Line::styled(prompt, prompt_style)), at);
    }
}

/// 麦克风画在框外面右边、和第一行文字同一行（2026-11-10 项目主人：原来是框里最右的一个点，「换成一个麦克风图标在输入框
/// 外部右侧更合理」）：不在听暗、在听红、停了等字转圈。交回画在哪（点它开始、停）；哪都放不下的不画。
fn draw_mic(
    frame: &mut Frame,
    areas: Areas,
    (mic, spin): (crate::voice::Mic, usize),
    config: &Config,
) -> Rect {
    use crate::voice::Mic;
    let limit = if areas.sidebar.width > 0 {
        areas.sidebar.x.saturating_sub(1)
    } else {
        frame.area().right()
    };
    let Some(x) = mic_column(areas, limit) else {
        return Rect::default();
    };
    let spinner = &config.timeline.spinner;
    let glyph = config.icons.mic.clone();
    let (text, style) = match mic {
        Mic::Idle => (glyph, theme::dim()),
        Mic::Listening => (glyph, theme::error()),
        Mic::Recognizing => {
            let frame_glyph = spinner.get(spin % spinner.len().max(1));
            (frame_glyph.cloned().unwrap_or(glyph), theme::accent())
        }
    };
    let at = Rect::new(x, areas.text.y, 1, 1);
    frame.render_widget(Paragraph::new(Line::styled(text, style)), at);
    at
}

/// 麦克风画在第几列：框右边线外面隔一列；外面只剩一列的（紧凑版面）贴着边线；外面一列都没有的退回框里文字右边
/// 留的那几列的头一格，再没有就不画。`limit` 是右边能画到哪一列以前（窗口的右边、侧边栏竖线的左边）。
fn mic_column(areas: Areas, limit: u16) -> Option<u16> {
    let edge = areas.frame.right();
    if edge + 1 < limit {
        Some(edge + 1)
    } else if edge < limit {
        Some(edge)
    } else {
        let inside = areas.text.right();
        (inside + 1 < edge).then_some(inside)
    }
}

/// 跑命令的模式里提示符换成的（「人自己跑命令 `!`」第 1 条）。
const SHELL_PROMPT: &str = "!";

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
pub(crate) fn styled_line<'a>(
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
    use super::{Areas, mic_column, styled_line};
    use ratatui::layout::Rect;

    #[test]
    fn the_microphone_sits_outside_the_box_on_the_right() {
        // 2026-11-10 项目主人：麦克风放到输入框外面右侧。框占 2..78 列，文字到 74 列。
        let areas = Areas {
            frame: Rect::new(2, 10, 76, 3),
            text: Rect::new(5, 11, 70, 1),
            ..Areas::default()
        };
        assert_eq!(mic_column(areas, 80), Some(79), "外面有两列：隔一列");
        assert_eq!(mic_column(areas, 79), Some(78), "外面只有一列：贴着边线");
        assert_eq!(mic_column(areas, 78), Some(75), "外面没有：退回框里");
        let tight = Areas {
            text: Rect::new(5, 11, 72, 1),
            ..areas
        };
        assert_eq!(mic_column(tight, 78), None, "框里也没留空的不画");
    }

    #[test]
    fn the_microphone_is_drawn_with_the_icon_sets_glyph_and_reports_where() {
        let config = crate::config::Config::builtin().unwrap();
        let areas = Areas {
            frame: Rect::new(2, 10, 76, 3),
            text: Rect::new(5, 11, 70, 1),
            ..Areas::default()
        };
        let backend = ratatui::backend::TestBackend::new(80, 14);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut at = Rect::default();
        terminal
            .draw(|f| at = super::draw_mic(f, areas, (crate::voice::Mic::Listening, 0), &config))
            .unwrap();
        assert_eq!(at, Rect::new(79, 11, 1, 1), "点这一格开始、停");
        let cell = &terminal.backend().buffer()[(79, 11)];
        assert_eq!(cell.symbol(), config.icons.mic, "图标那一套的麦克风");
        assert_eq!(cell.style().fg, theme::error().fg, "在听是红的");
        // 右边有侧边栏的不越过它的竖线。
        let beside = Areas {
            sidebar: Rect::new(79, 0, 1, 14),
            ..areas
        };
        terminal
            .draw(|f| at = super::draw_mic(f, beside, (crate::voice::Mic::Idle, 0), &config))
            .unwrap();
        assert_eq!(at.x, 75, "外面没地方：退回框里");
    }

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
