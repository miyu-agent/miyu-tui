//! 画抽屉（蓝图「确认和提问的抽屉」第 2、3 条）：谁在问、标签、选中的那一项、编辑的光标、补充、「确认」页、
//! 文字画的框、放不下时钉住问题和按键提示。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use unicode_width::UnicodeWidthStr;

use super::{fit, view};
use crate::config::Config;
use crate::drawer::{Asked, Drawer, Texts};

fn text(line: &ratatui::text::Line) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn lines(d: &Drawer, width: u16) -> Vec<String> {
    view(d, &texts(), &look(), width, false)
        .lines
        .iter()
        .map(text)
        .collect()
}

fn look() -> super::PreviewLook {
    Config::builtin().unwrap().layout.drawer_preview
}

fn texts() -> Texts {
    Config::builtin().unwrap().text.drawer
}

fn press(d: &mut Drawer, code: KeyCode) {
    d.key(KeyEvent::new(code, KeyModifiers::NONE));
}

/// 两道题：第一道 `n` 个选项（带说明），第二道没有短名。
fn asked(n: usize) -> Asked {
    let options: Vec<_> = (0..n)
        .map(|i| serde_json::json!({"label": format!("选项{i}"), "description": "说明"}))
        .collect();
    serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [
            {"header": "甲", "question": "第一道？", "options": options},
            {"question": "第二道？"}
        ]
    }))
    .unwrap()
}

#[test]
fn it_shows_who_asks_the_tabs_the_pointer_and_the_keys() {
    let d = Drawer::question(Some("子代理 查资料".into()), asked(2));
    let v = view(&d, &texts(), &look(), 60, false);
    let got: Vec<String> = v.lines.iter().map(text).collect();
    assert_eq!(got[0], "子代理 查资料 在问");
    assert_eq!(
        got[1], " 甲   第二道？  确认",
        "当前那个反色（两边空一格），没有短名的写问题，最后是「确认」"
    );
    assert_eq!(got[3], "第一道？");
    assert_eq!(got[5], "› 选项0", "选中的行首 ›");
    assert_eq!(got[6], "  说明", "说明和标题对齐");
    assert_eq!(got[7], "  选项1");
    assert_eq!(
        got[9], "  输入其他答案",
        "最后一项是「输入其他答案」，没有「不回答」"
    );
    assert_eq!(
        got.last().unwrap(),
        "↑/↓ 选择 · Enter 提交 · n 补充 · Esc ×2 取消 · ←/→ 换题"
    );
    assert_eq!(v.focus, (5, 6));
    assert_eq!((v.items[5], v.items[3]), (Some(0), None));
    // 标题加粗、选中的品红。
    let label = &v.lines[5].spans[1];
    assert!(
        label
            .style
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    assert_eq!(label.style.fg, crate::theme::picked().fg);
}

#[test]
fn other_edits_in_place_and_the_keys_say_only_save_and_leave() {
    let mut d = Drawer::question(None, asked(1));
    press(&mut d, KeyCode::Down);
    press(&mut d, KeyCode::Enter);
    let v = view(&d, &texts(), &look(), 60, false);
    let (row, col) = v.cursor.expect("编辑时有光标");
    assert_eq!(
        text(&v.lines[row]),
        "› 输入其他答案",
        "还没打字：暗色写提示"
    );
    assert_eq!(col, 2, "光标落在 › 后面");
    assert_eq!(text(v.lines.last().unwrap()), "Enter 保存 · Esc 退出编辑");
    press(&mut d, KeyCode::Char('好'));
    let v = view(&d, &texts(), &look(), 60, false);
    let (row, col) = v.cursor.unwrap();
    assert_eq!((text(&v.lines[row]).as_str(), col), ("› 好", 4));
    // 退出编辑：照写的字，不加「自定义：」（2026-10-20 项目主人）。
    press(&mut d, KeyCode::Esc);
    assert!(
        lines(&d, 60).contains(&"› 好".to_string()),
        "{:?}",
        lines(&d, 60)
    );
}

#[test]
fn notes_show_under_the_options() {
    let mut d = Drawer::question(None, asked(1));
    press(&mut d, KeyCode::Char('n'));
    for c in "快".chars() {
        press(&mut d, KeyCode::Char(c));
    }
    let v = view(&d, &texts(), &look(), 60, false);
    let (row, _) = v.cursor.unwrap();
    assert_eq!(text(&v.lines[row]), "  补充  快");
}

#[test]
fn the_review_page_lists_every_answer() {
    let mut d = Drawer::question(None, asked(1));
    press(&mut d, KeyCode::Enter);
    press(&mut d, KeyCode::Right);
    assert!(d.on_review());
    let got = lines(&d, 60);
    assert_eq!(got[0], "甲 ✓  第二道？   确认 ", "「确认」反色，答过的带 ✓");
    assert_eq!(got[2], "甲：选项0");
    assert_eq!(got[3], "第二道？：未回答");
    assert_eq!(got.last().unwrap(), "Enter 提交 · Esc ×2 取消 · ←/→ 换题");
}

#[test]
fn a_preview_sits_on_the_right_under_a_dim_title_and_below_when_narrow() {
    let asked: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [{"question": "哪种？", "options": [
            {"label": "左右", "preview": "┌──┬──┐\n│甲│乙│\n└──┴──┘"},
            {"label": "上下", "preview": "┌──┐\n└──┘"}
        ]}]
    }))
    .unwrap();
    let d = Drawer::question(None, asked);
    let wide = lines(&d, 100);
    // 竖线在中线（第 50 列），预览从第 52 列起，和选项顶上对齐：第一行暗色「预览」，下面是图；不铺底色，中间一条竖线（2026-10-07 项目主人）。
    assert!(wide[2].starts_with("› 左右"), "{wide:?}");
    let title = wide[2].find("预览").expect("第一行写预览");
    assert_eq!(wide[2][..title].width(), 52);
    let at = wide[3].find('┌').expect("图在下一行");
    assert_eq!(wide[3][..at].width(), 52);
    assert!(wide[5].contains("└──┴──┘"));
    assert!(!wide.iter().any(|l| l.contains('╭')), "不描边");
    for row in &wide[2..6] {
        let bar = row.find('│').expect("每行都有竖线");
        assert_eq!(row[..bar].width(), 50, "竖线在抽屉的中线上：{row}");
    }
    let v = view(&d, &texts(), &look(), 100, false);
    assert!(
        v.lines[3].spans.iter().all(|s| s.style.bg.is_none()),
        "不铺底色：{:?}",
        v.lines[3]
    );
    // 窄了：挪到选项下面，中间空一行。
    let narrow = lines(&d, 50);
    assert_eq!(narrow[2], "› 左右");
    let title = narrow.iter().position(|l| l == "预览").expect("有预览");
    assert_eq!(narrow[title - 1], "", "{narrow:?}");
    assert_eq!(narrow[title + 1], "┌──┬──┐", "{narrow:?}");
    assert!(!narrow[2].contains('│'), "上下排不画竖线");
}

#[test]
fn a_tall_drawer_scrolls_the_options_and_pins_the_question_and_keys() {
    let mut d = Drawer::question(None, asked(12));
    for _ in 0..12 {
        press(&mut d, KeyCode::Down);
    }
    let full = view(&d, &texts(), &look(), 60, false);
    let keys = text(full.lines.last().unwrap());
    let mut scroll = 0;
    let v = fit(full, 10, &mut scroll);
    let got: Vec<String> = v.lines.iter().map(text).collect();
    assert_eq!(got.len(), 10);
    assert_eq!(got[9], keys, "按键提示钉在最下面");
    assert_eq!(got[2], "第一道？", "问题钉在上面，不滚走");
    assert!(
        got.contains(&"› 输入其他答案".to_string()),
        "选中的那一项露出来"
    );
    assert!(scroll > 0);
}

#[test]
fn a_very_short_drawer_keeps_the_picked_option_before_the_blank_line() {
    // 2026-09-29 30×7 实测：抽屉只剩问题的最后一行、空行、按键提示，选项一行都没有（「窗口小的时候」第 3 条）。
    let d = Drawer::question(None, asked(4));
    let full = view(&d, &texts(), &look(), 60, false);
    let keys = text(full.lines.last().unwrap());
    let picked = full
        .lines
        .iter()
        .map(text)
        .find(|l| l.starts_with("› "))
        .unwrap();
    let fitted = |height: usize| -> Vec<String> {
        let mut scroll = 0;
        fit(view(&d, &texts(), &look(), 60, false), height, &mut scroll)
            .lines
            .iter()
            .map(text)
            .collect()
    };
    let three = fitted(3);
    assert_eq!(three.len(), 3);
    assert_eq!(three[1], picked, "先去空行，留一行选项：{three:?}");
    assert_eq!(three[2], keys);
    assert_eq!(fitted(2), [picked.clone(), keys], "再去问题");
    assert_eq!(
        fitted(1),
        std::slice::from_ref(&picked),
        "最后连按键提示也去掉"
    );
    // 四行起照旧：问题、选项、空行、按键提示。
    let four = fitted(4);
    assert_eq!((four[1].as_str(), four[2].as_str()), (picked.as_str(), ""));
}

#[test]
fn the_first_esc_turns_the_keys_line_into_a_warning() {
    use std::time::{Duration, Instant};
    let mut d = Drawer::question(None, asked(1));
    d.escape(Instant::now(), Duration::from_secs(60));
    let v = view(&d, &texts(), &look(), 60, false);
    let last = v.lines.last().unwrap();
    assert_eq!(text(last), "再按一次 Esc 取消");
    assert_eq!(last.style.fg, crate::theme::warn().fg);
}

#[test]
fn installing_a_package_shows_what_it_is_under_the_question() {
    // 2026-10-12 项目主人定：她用 `package` 工具装包时，照 `miyu pkg install` 列清楚（核心 F-10 下，`access` 是 `install`）。
    let approval = serde_json::from_value(serde_json::json!({
        "call_id": "c1", "access": "install",
        "detail": {"tool": "package", "install": "/home/alice/weather", "preview": {
            "package": "weather", "version": "1.2.0", "program": "process",
            "capabilities": [{"name": "联网"}], "files": 3, "size": 4096}}
    }))
    .unwrap();
    let d = Drawer::approval(None, approval);
    let shown = lines(&d, 60);
    let at = |text: &str| shown.iter().position(|l| l.trim() == text);
    assert_eq!(at("安装 weather 1.2.0"), Some(0), "{shown:#?}");
    assert_eq!(at("包含：扩展程序"), Some(1), "{shown:#?}");
    assert_eq!(at("需要：联网"), Some(2), "{shown:#?}");
    assert_eq!(at("大小：4.0 KiB，3 个文件"), Some(3), "{shown:#?}");
    assert_eq!(d.items(0).len(), 2, "没有 rule：只有允许本次、不允许");
}

#[test]
fn a_long_path_is_clipped_before_its_mark_so_the_mark_stays() {
    // 2026-10-22 走查：工作区路径长的，「受保护」这类标记被挤出行外；路径先截，标记照写。
    let long = format!("/srv/{}/.envrc", "deep/".repeat(20));
    let approval = serde_json::from_value(serde_json::json!({
        "call_id": "c1", "access": "write",
        "detail": {"tool": "write", "paths": [{"path": long, "write": true, "zone": "read_only"}]}
    }))
    .unwrap();
    let d = Drawer::approval(None, approval);
    let shown = lines(&d, 40);
    let row = shown
        .iter()
        .find(|l| l.contains("/srv/"))
        .unwrap_or_else(|| panic!("{shown:#?}"));
    assert!(row.ends_with("… 受保护"), "{row}");
    assert!(row.width() <= 40, "{row}");
}

/// 确认跑一条 `n` 行的命令（第几行写 `echo 第几行`），短标题「跑探针」。
fn long_command(n: usize) -> Drawer {
    let approval = serde_json::from_value(serde_json::json!({
        "call_id": "c1", "access": "execute", "detail": {"tool": "shell"}
    }))
    .unwrap();
    let mut d = Drawer::approval(None, approval);
    let command = (1..=n)
        .map(|i| format!("echo {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    d.command = Some((Some("跑探针".into()), command));
    d
}

#[test]
fn a_long_command_shows_a_window_of_it_and_scrolls() {
    // 2026-10-22 项目主人：命令太长把问题顶没了，照选「全文放在可以滚的一块里」；看了样子又定「标题和提示在上、命令一块
    // 缩进」：问题下面空一行，命令每行前面一根竖线，放不下的块下面写还有几行。
    let mut d = long_command(30);
    let shown = lines(&d, 60);
    assert_eq!(shown[0], "跑探针", "{shown:#?}");
    assert_eq!(shown[1], "", "问题和命令之间空一行：{shown:#?}");
    let commands: Vec<&String> = shown.iter().filter(|l| l.contains("$ echo")).collect();
    assert_eq!(commands.len(), 8, "只露 8 行：{shown:#?}");
    assert_eq!(commands[0], "│ $ echo 1");
    assert!(shown.iter().any(|l| l == "↓ 还有 22 行"), "{shown:#?}");
    d.command_rows = 30;
    for _ in 0..3 {
        d.scroll_command(true);
    }
    let shown = lines(&d, 60);
    assert!(shown.iter().any(|l| l == "│ $ echo 4"), "{shown:#?}");
    assert!(!shown.iter().any(|l| l == "│ $ echo 3"), "{shown:#?}");
    assert!(
        shown.iter().any(|l| l == "↑ 上面还有 3 行 · ↓ 还有 19 行"),
        "{shown:#?}"
    );
    assert!(
        shown
            .last()
            .is_some_and(|l| l.contains("Ctrl+J/K 滚动命令")),
        "能滚的按键提示里写：{shown:#?}"
    );
    for _ in 0..40 {
        d.scroll_command(true);
    }
    assert_eq!(d.command_top, 22, "停在最后一段");
    let short = long_command(3);
    let shown = lines(&short, 60);
    assert!(
        !shown
            .iter()
            .any(|l| l.contains("还有") || l.contains("Ctrl+J/K")),
        "放得下的不写：{shown:#?}"
    );
}

#[test]
fn a_wrapped_command_line_is_indented_under_the_dollar() {
    let mut d = long_command(1);
    d.command = Some((None, format!("probe {}", "x".repeat(30))));
    let shown = lines(&d, 20);
    let at = shown
        .iter()
        .position(|l| l.starts_with("│ $ probe"))
        .unwrap();
    assert!(shown[at + 1].starts_with("│   x"), "{shown:#?}");
}

#[test]
fn ctrl_j_k_scroll_the_command_unless_writing_where_ctrl_j_is_a_new_line() {
    // 同一天项目主人：「ctrl+JK怎么不能滚」。
    let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
    let mut d = long_command(30);
    d.command_rows = 30;
    d.key(ctrl('j'));
    d.key(ctrl('j'));
    d.key(ctrl('k'));
    assert_eq!(d.command_top, 1);
    // 落到「不允许」上就在写理由：Ctrl+J 是换行。
    press(&mut d, KeyCode::Down);
    press(&mut d, KeyCode::Down);
    assert_eq!(d.editing, Some(crate::drawer::Edit::Reason));
    d.key(ctrl('j'));
    assert_eq!((d.command_top, d.typed[0].as_str()), (1, "\n"));
}

#[test]
fn a_reason_with_a_new_line_shows_on_two_rows() {
    // 同一天项目主人：「理由不支持换行」（换行原来写成 `↲` 挤在一行里）。
    let mut d = long_command(1);
    press(&mut d, KeyCode::Down);
    press(&mut d, KeyCode::Down);
    d.typed[0] = "第一行\n第二行".into();
    let v = view(&d, &texts(), &look(), 40, false);
    let shown: Vec<String> = v.lines.iter().map(text).collect();
    let at = shown
        .iter()
        .position(|l| l.ends_with("第一行"))
        .expect("第一行接在不允许后面");
    assert!(shown[at].contains("不允许"), "{shown:#?}");
    // 比显示的列（中文一个字两列、三个字节），不比字节。
    let col = |line: &str, word: &str| line.find(word).map(|i| line[..i].width());
    assert_eq!(
        col(&shown[at + 1], "第二行"),
        col(&shown[at], "第一行"),
        "对齐接着写：{shown:#?}"
    );
    assert_eq!(v.cursor.map(|(row, _)| row), Some(at + 1), "光标在最后一行");
}

#[test]
fn a_short_drawer_keeps_the_confirm_title() {
    // 同一天：放不下时问题行、「高危命令」不被顶掉。
    let mut d = long_command(30);
    if let crate::drawer::Ask::Approval(a) = &mut d.ask {
        a.detail = serde_json::from_value(
            serde_json::json!({"tool": "shell", "hazard": "delete_unresolved"}),
        )
        .ok();
    }
    let mut scroll = 0;
    let v = fit(view(&d, &texts(), &look(), 60, false), 9, &mut scroll);
    let shown: Vec<String> = v.lines.iter().map(text).collect();
    assert_eq!(shown[0], "跑探针", "{shown:#?}");
    assert_eq!(shown[1], "高危命令", "{shown:#?}");
    assert!(
        shown.iter().any(|l| l.contains("不允许")),
        "选项还在：{shown:#?}"
    );
}

#[test]
fn a_short_drawer_shows_fewer_command_rows_but_keeps_the_blank_lines() {
    // 2026-10-22 项目主人看了样子：「空行也没有」——抽屉放不下时原来先裁空行，命令照样露 8 行。
    let mut d = long_command(30);
    let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 14)).unwrap();
    let mut caret = crate::caret::Caret::default();
    term.draw(|f| {
        super::draw(f, f.area(), &mut d, &texts(), &look(), false, &mut caret);
    })
    .unwrap();
    let buf = term.backend().buffer();
    let shown: Vec<String> = (0..14)
        .map(|y| {
            let mut line = String::new();
            let mut skip = 0;
            for x in 0..60 {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let symbol = buf[(x, y)].symbol();
                skip = symbol.width().saturating_sub(1);
                line.push_str(symbol);
            }
            line.trim_end().to_string()
        })
        .collect();
    assert_eq!(shown[0], "跑探针", "{shown:#?}");
    assert_eq!(shown[1], "", "问题和命令之间的空行留着：{shown:#?}");
    let commands = shown.iter().filter(|l| l.contains("$ echo")).count();
    assert!((2..8).contains(&commands), "少露几行命令：{shown:#?}");
    let more = shown
        .iter()
        .position(|l| l.starts_with("↓ 还有"))
        .expect("写还有几行");
    assert_eq!(shown[more + 1], "", "命令和选项之间的空行留着：{shown:#?}");
    assert!(d.command_scrolls(), "照收窄的窗口算能滚");
}

mod preview;
