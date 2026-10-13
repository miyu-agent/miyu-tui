//! 配置页画出来的样子：TestBackend 上看字、看光标，不连核心。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Position;
use ratatui::{Terminal, backend::TestBackend};
use serde_json::json;

use super::draw;
use crate::caret::Caret;
use crate::config::Config;
use crate::settings::Settings;

fn page(config: &Config) -> Settings {
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => crate::settings::test_support::model_list(),
            "config.get" => crate::settings::test_support::config_all(),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    page
}

fn press(page: &mut Settings, config: &Config, code: KeyCode) {
    page.key(
        KeyEvent::new(code, KeyModifiers::NONE),
        &config.text.settings,
    );
}

fn screen(page: &mut Settings, config: &Config, width: u16, height: u16) -> (String, Caret) {
    let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut caret = Caret::default();
    term.draw(|frame| draw(frame, page, config, &mut caret))
        .unwrap();
    let buf = term.backend().buffer();
    // 宽字后面那一格是占位的，拼字时跳过。
    let text = (0..height)
        .map(|y| {
            let mut line = String::new();
            let mut skip = false;
            for x in 0..width {
                let symbol = buf[(x, y)].symbol();
                if !std::mem::take(&mut skip) {
                    line.push_str(symbol);
                    skip = unicode_width::UnicodeWidthStr::width(symbol) == 2;
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n");
    (text, caret)
}

#[test]
fn the_menu_then_three_columns_with_org_filter_and_counter() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    let (menu, _) = screen(&mut p, &config, 120, 30);
    assert!(menu.contains("● 配置"));
    assert!(menu.contains("供应商和模型"));
    press(&mut p, &config, KeyCode::Enter);
    press(&mut p, &config, KeyCode::Down);
    let (text, _) = screen(&mut p, &config, 120, 30);
    assert!(text.contains("◉ 供应商和模型"));
    for want in ["供应商", "组织", "模型", "中转站", "全部", "cline", "其他"] {
        assert!(text.contains(want), "少了「{want}」：\n{text}");
    }
    assert!(
        !text.contains("1M"),
        "模型栏右边不写窗口（2026-10-07 项目主人）：\n{text}"
    );
    assert!(text.contains("2/2"), "第二行右下角写第几个：\n{text}");
}

#[test]
fn a_model_form_masks_nothing_but_keys_and_shows_sources() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    press(&mut p, &config, KeyCode::Enter);
    press(&mut p, &config, KeyCode::Down);
    press(&mut p, &config, KeyCode::Enter);
    let (text, _) = screen(&mut p, &config, 120, 34);
    assert!(text.contains("编辑供应商 · 中转站"));
    assert!(text.contains("已保存到密钥库"), "key 不显示值：\n{text}");
    // 已有的 ID 只读，开窗停在显示名称：往下四行是 key。
    for _ in 0..4 {
        press(&mut p, &config, KeyCode::Down);
    }
    press(&mut p, &config, KeyCode::Enter);
    for c in "SECRET-VALUE".chars() {
        press(&mut p, &config, KeyCode::Char(c));
    }
    let (text, caret) = screen(&mut p, &config, 120, 34);
    assert!(!text.contains("SECRET-VALUE"), "打码：\n{text}");
    assert!(text.contains("••••"));
    assert!(caret.shown, "正在改：光标露出来");
}

#[test]
fn narrow_and_tiny_terminals_do_not_panic_and_clicks_select_rows() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    press(&mut p, &config, KeyCode::Enter);
    for (w, h) in [(1, 1), (20, 5), (60, 12), (90, 24)] {
        screen(&mut p, &config, w, h);
    }
    screen(&mut p, &config, 120, 30);
    let (_, row) = p
        .hits
        .iter()
        .find(|(_, hit)| *hit == crate::settings::Hit::Row(crate::settings::nav::Col::Provider, 1))
        .copied()
        .map(|(r, h)| (h, r))
        .unwrap();
    p.click(
        Position::new(row.x + 3, row.y),
        std::time::Duration::ZERO,
        &config.text.settings,
    );
    assert_eq!(p.nav.provider(&p.view).unwrap().id, "relay");
}

#[test]
fn a_form_with_an_error_on_a_very_short_terminal_does_not_panic() {
    // review A7（2026-10-11）：弹窗的报错写在按钮那一行上面一行；终端只有四五行高时按钮那一行算到 0，减 1 下溢崩溃。
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    press(&mut p, &config, KeyCode::Enter);
    press(&mut p, &config, KeyCode::Down);
    press(&mut p, &config, KeyCode::Enter);
    if let Some(crate::settings::popup::Popup::Form(form)) = &mut p.popup {
        form.error = Some("存不进去".into());
    } else {
        panic!("开了供应商的窗");
    }
    for (w, h) in [(40, 3), (40, 4), (40, 5), (60, 8), (120, 34)] {
        screen(&mut p, &config, w, h);
    }
}

#[test]
fn a_long_menu_note_widens_the_entry_and_is_cut_with_an_ellipsis_only_when_the_window_is_narrow() {
    // 2026-10-13 验收 T-176：通用底下的组名多，原来 46 列处硬切成「…终端界面、时间」，不加「…」。
    let config = Config::builtin().unwrap();
    let mut page = page(&config);
    let groups = [
        "显示",
        "人格",
        "预设",
        "会话",
        "压缩",
        "终端界面",
        "时间线",
        "吉祥物",
    ];
    let group_json: Vec<_> = groups
        .iter()
        .enumerate()
        .map(|(i, name)| json!({"id": format!("g{i}"), "name": name, "page": "general"}))
        .collect();
    let items: Vec<_> = (0..groups.len())
        .map(|i| json!({"key": format!("ui.k{i}"), "control": "toggle", "layers": ["personal"], "name": "项", "page": "general", "group": format!("g{i}")}))
        .collect();
    page.more.schema = Some(crate::settings::pages::schema::Schema::read(&json!({
        "pages": [{"id": "general", "name": "通用"}],
        "groups": group_json,
        "items": items,
    })));
    let whole = groups.join("、");
    let (wide, _) = screen(&mut page, &config, 140, 30);
    assert!(wide.contains(&whole), "宽窗口整行写得下：\n{wide}");
    let (narrow, _) = screen(&mut page, &config, 44, 30);
    assert!(!narrow.contains(&whole));
    assert!(narrow.contains("…"), "放不下的截掉加「…」：\n{narrow}");
}

#[test]
fn a_long_form_error_wraps_instead_of_being_cut() {
    // 2026-10-13 验收 T-181：「「Bad_ID」无效：…最长 64 个字符」在浮窗里一行截掉，最后那半句看不到。
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    press(&mut p, &config, KeyCode::Enter);
    press(&mut p, &config, KeyCode::Down);
    press(&mut p, &config, KeyCode::Enter);
    let error = "「Bad_ID」无效：须以小写字母开头，仅含小写字母、数字、-、_，最长 64 个字符";
    if let Some(crate::settings::popup::Popup::Form(form)) = &mut p.popup {
        form.error = Some(error.into());
    } else {
        panic!("开了供应商的窗");
    }
    let (text, _) = screen(&mut p, &config, 140, 40);
    let flat: String = text.lines().map(str::trim).collect();
    assert!(flat.contains("最长 64 个字符"), "报错折行写全：\n{text}");
    assert!(!text.contains('…'), "写得下就不截：\n{text}");
}
