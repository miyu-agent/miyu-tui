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

#[test]
fn a_secret_being_typed_is_drawn_as_dots() {
    // 「配置页」第 48 条：密钥的编辑窗里一个字一个 `•`，明文不上屏。
    use crate::settings::pages::choose::LineEdit;
    use crate::settings::pages::target::Target;
    use crate::settings::popup::Popup;
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    let mut edit = LineEdit::open("im.token", "令牌", false, &serde_json::Value::Null);
    edit.editor.set("t0ken");
    p.popup = Some(Popup::Line(edit.to(Target::Secret)));
    let (text, _) = screen(&mut p, &config, 120, 30);
    assert!(text.contains("令牌") && text.contains("•••••"), "{text}");
    assert!(!text.contains("t0ken"), "明文不上屏：\n{text}");
}

#[test]
fn the_model_form_sets_the_prices_apart_and_drops_their_default_tags() {
    // 2026-10-19 项目主人：价格那几行右边的「默认」都去掉；「价格 · 每百万 token」和上面「温度」之间空一行。
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    press(&mut p, &config, KeyCode::Enter);
    press(&mut p, &config, KeyCode::Right);
    press(&mut p, &config, KeyCode::Enter);
    let (text, _) = screen(&mut p, &config, 120, 40);
    let lines: Vec<&str> = text.lines().collect();
    let at = |s: &str| {
        lines
            .iter()
            .position(|l| l.contains(s))
            .unwrap_or_else(|| panic!("{s}：\n{text}"))
    };
    let price = at("价格 · 每百万 token");
    assert!(
        lines[price - 1].trim().is_empty(),
        "价格上面空一行：\n{text}"
    );
    assert!(lines[price - 2].contains("温度"), "{text}");
    for name in ["输入", "输出", "缓存读", "缓存写", "币种", "倍率"] {
        let row = lines[price..]
            .iter()
            .find(|l| l.trim_start().starts_with(name))
            .unwrap_or_else(|| panic!("{name}：\n{text}"));
        assert!(!row.contains("默认"), "{name} 不写默认：\n{text}");
    }
    assert!(
        lines[at("默认思考")].trim_end().ends_with("默认"),
        "别的照旧写来源：\n{text}"
    );
}

#[test]
fn typing_in_a_form_field_is_not_underlined() {
    // 2026-10-19 项目主人：模型、供应商的编辑框里打字有下划线。
    use ratatui::style::Modifier;
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    press(&mut p, &config, KeyCode::Enter);
    press(&mut p, &config, KeyCode::Down);
    press(&mut p, &config, KeyCode::Enter);
    // 开窗停在显示名称：回车开始改，打几个字。
    press(&mut p, &config, KeyCode::Enter);
    for c in "QWZ".chars() {
        press(&mut p, &config, KeyCode::Char(c));
    }
    let mut term = Terminal::new(TestBackend::new(120, 34)).unwrap();
    let mut caret = Caret::default();
    term.draw(|frame| draw(frame, &mut p, &config, &mut caret))
        .unwrap();
    let buf = term.backend().buffer();
    let typed: Vec<_> = buf.content().iter().filter(|c| c.symbol() == "Q").collect();
    assert!(!typed.is_empty(), "打的字画出来了");
    assert!(
        typed
            .iter()
            .all(|c| !c.modifier.contains(Modifier::UNDERLINED)),
        "不加下划线"
    );
}

#[test]
fn price_rows_written_by_hand_do_not_say_personal_but_other_rows_still_do() {
    // 2026-10-19 项目主人：价格那几行右边的「个人」也去掉。
    use crate::settings::forms::{Field, Row, Val};
    let config = Config::builtin().unwrap();
    let texts = &config.text.settings;
    let own = |field| Row {
        field,
        val: Val::Text {
            text: "1.0".into(),
            secret: false,
        },
        inherit: Some("2.0".into()),
        source: Some("catalog".into()),
    };
    for field in [Field::Input, Field::Currency, Field::Multiplier] {
        assert_eq!(super::fields::source(&own(field), texts).0, "", "{field:?}");
    }
    assert_eq!(
        super::fields::source(&own(Field::Window), texts).0,
        texts.source("personal")
    );
}

#[test]
fn an_unset_secret_reads_dim_and_a_set_one_stays_bold() {
    // 2026-10-19 项目主人：联网搜索页几行 key 都是「未设置」，暗着写。
    use ratatui::style::Modifier;
    let config = Config::builtin().unwrap();
    let texts = &config.text.settings;
    let opened = |value: serde_json::Value| {
        let schema = json!({"pages": [{"id": "general", "name": "通用"}],
            "groups": [{"id": "net", "name": "联网搜索", "page": "general"}],
            "items": [{"key": "net.search.tavily.key", "type": "secret", "control": "text",
                "layers": ["personal"], "name": "Tavily", "page": "general", "group": "net"}]});
        let layers = if value.is_null() {
            json!([])
        } else {
            json!([{"origin": {"layer": "personal", "line": 1}, "value": value, "used": true}])
        };
        let data = json!({"items": {"net.search.tavily.key": {"value": value, "layers": layers}}});
        let mut page = Settings::open(false, true);
        for (tag, method, _) in page.take_asks() {
            let got = match method {
                "config.schema" => schema.clone(),
                "config.get" => data.clone(),
                _ => json!({}),
            };
            page.answer(tag, Ok(got), texts);
        }
        page
    };
    let value_span = |page: &Settings, word: &str| {
        super::pages::page_lines(page, "general", 0, texts, 60)
            .into_iter()
            .flat_map(|(line, _)| line.spans)
            .find(|s| s.content.contains(word))
            .unwrap_or_else(|| panic!("没写 {word}"))
    };
    let unset = value_span(&opened(serde_json::Value::Null), "未设置");
    assert_eq!(unset.style.fg, crate::theme::dim().fg, "暗色");
    assert!(!unset.style.add_modifier.contains(Modifier::BOLD), "不加粗");
    let set = value_span(&opened(json!({"secret": "tavily-key"})), "已设置");
    assert!(
        set.style.add_modifier.contains(Modifier::BOLD),
        "设了的照常加粗"
    );
}

#[test]
fn a_package_page_has_a_gap_under_its_origin_line_and_the_upstream_is_a_link() {
    // 2026-10-20 项目主人：那一行下面空一行；链接点得开。
    use crate::settings::Hit;
    use crate::settings::pages::Section;
    let config = Config::builtin().unwrap();
    let mut page = Settings::open(false, true);
    let item = json!({"key": "im.listen", "name": "端口", "page": "packages", "group": "im",
        "package": "im", "control": "text", "kind": "string", "layers": ["system"]});
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => crate::settings::test_support::model_list(),
            "config.get" => crate::settings::test_support::config_all(),
            "config.schema" => json!({"pages": [{"id": "packages", "name": "软件包"}],
                "groups": [{"id": "im", "name": "接入通讯平台", "page": "packages"}],
                "items": [item]}),
            "package.list" => json!({"packages": [{"package": "im", "name": "接入通讯平台",
                "author": "shorin", "from": "official", "upstream": "https://github.com/miyu-agent/miyu-im"}]}),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    page.on_menu = false;
    page.more.section = Some(Section::Page("packages/im".into(), 0));
    let (text, _) = screen(&mut page, &config, 80, 20);
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.contains("github.com/miyu-agent/miyu-im"))
        .expect("那一行");
    assert!(lines[at].contains("shorin · 官方软件源 · "), "{text}");
    assert!(lines[at - 2].contains("软件包 › 接入通讯平台"), "{text}");
    assert_eq!(lines[at - 1].trim(), "", "和面包屑之间空一行：\n{text}");
    assert_eq!(lines[at + 1].trim(), "", "下面空一行：\n{text}");
    assert!(lines[at + 2].contains("端口"), "项从空行下面起：\n{text}");
    let link = page
        .hits
        .iter()
        .find(|(_, h)| *h == Hit::Link)
        .expect("上游能点");
    // 照列数：前面的汉字一个占两列。
    let (start, width) = (usize::from(link.0.x), usize::from(link.0.width));
    let mut col = 0;
    let shown: String = lines[at]
        .chars()
        .filter(|c| {
            let here = col;
            col += unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0);
            here >= start && here < start + width
        })
        .collect();
    assert_eq!(shown, "github.com/miyu-agent/miyu-im", "点的就是上游那一截");
    assert_eq!(
        page.link.as_deref(),
        Some("https://github.com/miyu-agent/miyu-im")
    ); // 指针停上去：下划线、指针变手（2026-10-20 项目主人：没有悬浮效果、光标不会变手）。
    assert!(!page.over_link());
    let rect = link.0;
    page.hover = Some(Position::new(rect.x + 1, rect.y));
    assert!(page.over_link(), "停在上游上");
    let mut term = Terminal::new(TestBackend::new(80, 20)).unwrap();
    let mut caret = Caret::default();
    term.draw(|frame| draw(frame, &mut page, &config, &mut caret))
        .unwrap();
    let cell = &term.backend().buffer()[(rect.x + 1, rect.y)];
    assert!(
        cell.modifier.contains(ratatui::style::Modifier::UNDERLINED),
        "悬停加下划线"
    );
}

mod catalog;
mod presets;

#[test]
fn the_fixed_pools_are_named_and_listed_first() {
    // 2026-10-26 项目主人：固定的三个池要有显示名称（旗舰、日常、轻量），排在所有池最前面。
    let config = Config::builtin().unwrap();
    let mut list = crate::settings::test_support::model_list();
    list["pools"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "lite", "strategy": "pin", "models": [], "fixed": true}));
    // 旁路请求登记（核心 Q-2）：起标题走轻量。
    list["side"] = json!([{"id": "core/title", "tier": "lite", "name": "起标题", "summary": "", "resolved": "session"}]);
    let mut p = Settings::open(false, true);
    for (tag, method, _) in p.take_asks() {
        let got = match method {
            "model.list" => list.clone(),
            "config.get" => crate::settings::test_support::config_all(),
            _ => json!({"secrets": []}),
        };
        p.answer(tag, Ok(got), &config.text.settings);
    }
    press(&mut p, &config, KeyCode::Enter);
    p.nav.page_step(true);
    p.nav.page_step(true);
    let (text, _) = screen(&mut p, &config, 120, 30);
    let lite = text
        .find("轻量")
        .unwrap_or_else(|| panic!("写显示名称：\n{text}"));
    let daily = text.find("daily").unwrap_or_else(|| panic!("{text}"));
    assert!(lite < daily, "固定的排前面：\n{text}");
    let row = text
        .lines()
        .find(|l| l.contains("轻量"))
        .unwrap_or_default();
    assert!(
        row.contains("用于：起标题"),
        "走这一档的旁路请求写在池右边：{row}"
    );
}
