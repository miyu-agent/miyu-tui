//! 「MCP」那一页画出来的样子（「配置页」第 43b 条）：要登录的那一行右边有「登录」，点得到、指针停上去变手。

use ratatui::layout::Position;
use ratatui::{Terminal, backend::TestBackend};
use serde_json::json;

use crate::caret::Caret;
use crate::config::Config;
use crate::settings::pages::Section;
use crate::settings::{Hit, Settings};
use crate::ui::settings::draw;

fn page(config: &Config) -> Settings {
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => crate::settings::test_support::model_list(),
            "config.get" => crate::settings::test_support::config_all(),
            "mcp.list" => json!({"servers": [
                {"name": "notes", "state": "stopped", "reason": "needs_login"},
                {"name": "context7", "state": "running", "tools": ["a", "b"]}]}),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    page.on_menu = false;
    page.more.section = Some(Section::Mcp(1));
    page
}

fn screen(page: &mut Settings, config: &Config) -> Vec<String> {
    let (width, height) = (60, 16);
    let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut caret = Caret::default();
    term.draw(|frame| draw(frame, page, config, &mut caret))
        .unwrap();
    let buf = term.backend().buffer();
    (0..height)
        .map(|y| {
            let mut line = String::new();
            let mut skip = 0;
            for x in 0..width {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let symbol = buf[(x, y)].symbol();
                skip = unicode_width::UnicodeWidthStr::width(symbol).saturating_sub(1);
                line.push_str(symbol);
            }
            line
        })
        .collect()
}

#[test]
fn a_server_that_needs_login_has_a_login_button_on_its_row() {
    let config = Config::builtin().unwrap();
    let mut page = page(&config);
    let lines = screen(&mut page, &config);
    let notes = lines
        .iter()
        .position(|l| l.contains("notes"))
        .expect("notes 那一行");
    let row = &lines[notes];
    assert!(
        row.contains("要登录")
            && row.matches("登录").count() == 2
            && row.trim_end().ends_with("登录"),
        "{lines:#?}"
    );
    let running = &lines[notes + 1];
    assert!(
        running.contains("context7") && running.contains("在跑 · 2 件工具"),
        "{lines:#?}"
    );
    assert!(!running.contains("登录"), "在跑的没有按钮");
    let logins: Vec<_> = page
        .hits
        .iter()
        .filter(|(_, h)| matches!(h, Hit::Login(_)))
        .collect();
    assert_eq!(logins.len(), 1, "只有 notes 有");
    let (area, hit) = *logins[0];
    assert_eq!(hit, Hit::Login(0));
    assert_eq!(usize::from(area.y), notes, "按钮就在那一行");
    // 指针停上去变手。
    assert!(!page.over_link());
    page.hover = Some(Position::new(area.x, area.y));
    assert!(page.over_link());
}

#[test]
fn clicking_login_selects_the_row_and_asks_for_the_sign_in_page() {
    let config = Config::builtin().unwrap();
    let mut page = page(&config);
    screen(&mut page, &config);
    let area = page
        .hits
        .iter()
        .find(|(_, h)| matches!(h, Hit::Login(_)))
        .map(|(r, _)| *r)
        .unwrap();
    let window = std::time::Duration::from_millis(400);
    page.click(
        Position::new(area.x + 1, area.y),
        window,
        &config.text.settings,
    );
    assert_eq!(page.more.section, Some(Section::Mcp(0)), "选中 notes");
    let asks = page.take_asks();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].1, "mcp.login");
    assert_eq!(asks[0].2, json!({"service": "notes"}));
}
