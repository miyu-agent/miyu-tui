//! 模型池那一页（「配置页」，2026-10-27 项目主人）：空池不写「空池」；「用于」放不下截掉，选中时成员栏下面写全。

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{press, screen};
use crate::config::Config;
use crate::settings::Settings;
use crate::settings::forms::Focus;
use crate::settings::popup::Popup;

/// 池的三种分法写成做法（2026-10-30 项目主人：「这三个各自是什么意思其实有些不太明朗，用户不知道该选哪个」）。
#[test]
fn pool_strategies_say_what_they_do() {
    let options = &Config::builtin().unwrap().text.settings.options;
    let said = |k: &str| options.get(k).map(String::as_str);
    assert_eq!(said("pin"), Some("按会话固定"));
    assert_eq!(said("rotate"), Some("每次请求轮换"));
    assert_eq!(said("ordered"), Some("不可用时回退"));
}

#[test]
fn a_pool_row_leaves_its_uses_to_the_list_below_its_members() {
    let config = Config::builtin().unwrap();
    let mut list = crate::settings::test_support::model_list();
    list["pools"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "lite", "strategy": "pin", "models": [], "fixed": true}));
    let side = |id: &str, name: &str| json!({"id": id, "tier": "lite", "name": name, "summary": "", "resolved": "session"});
    list["side"] = json!([
        side("core/title", "起标题"),
        side("memory/organize", "整理记忆"),
        side("core/recap", "回顾这一轮做了什么"),
        side("im/glance", "看一眼群里在聊什么")
    ]);
    // 扩展包登记的带 `package`（发版前 check 29）。
    list["side"][3]["package"] = json!("im");
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
    let row = text
        .lines()
        .find(|l| l.contains("轻量"))
        .unwrap_or_default()
        .to_string();
    let empty = text
        .lines()
        .find(|l| l.contains("empty"))
        .unwrap_or_default();
    assert!(!empty.trim_end().ends_with("空池"), "空池不写：{empty}");
    // 2026-10-29 项目主人：成员栏下面已经写了「用于」，池这一行不再写。
    assert!(!row.contains("用于") && !row.contains("起标题"), "{row}");
    // 2026-10-30 项目主人：分法写成做法，看得懂选哪个。
    let daily = text
        .lines()
        .find(|l| l.contains("daily"))
        .unwrap_or_default();
    assert!(daily.contains("每次请求轮换"), "{daily}");
    // 选中的就是轻量（固定的排最前、只有它）：成员栏下面写全。
    for name in [
        "用于",
        "起标题",
        "整理记忆",
        "回顾这一轮做了什么",
        "看一眼群里在聊什么（im）",
    ] {
        assert!(
            text.lines().any(|l| l.trim_end().ends_with(name)),
            "成员栏下面写全「{name}」：\n{text}"
        );
    }
}

/// 发版前 check 第 30 条（2026-11-09 项目主人）：编辑模型池的窗，按钮里暗着写快捷键，长列表不用挪到最底下也知道按 `s` 存。
#[test]
fn the_pool_form_buttons_show_their_keys() {
    let config = Config::builtin().unwrap();
    let mut list = crate::settings::test_support::model_list();
    list["pools"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "lite", "strategy": "pin", "models": [], "fixed": true}));
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
    press(&mut p, &config, KeyCode::Enter);
    let (text, _) = screen(&mut p, &config, 120, 30);
    assert!(text.contains("编辑模型池"), "开了编辑窗：\n{text}");
    let buttons = text
        .lines()
        .map(|l| l.replace(' ', ""))
        .find(|l| l.contains("[保存"))
        .unwrap_or_default();
    assert!(
        buttons.contains("[保存s]") && buttons.contains("[取消Esc]"),
        "按钮里写着快捷键：\n{text}"
    );
}

/// 模型多的样例：dev 多 30 个模型，固定的「轻量」池勾了两个（先 relay 的、再 dev/flash），开它的编辑窗。
fn crowded_pool_form(height: u16) -> (Settings, Config) {
    let config = Config::builtin().unwrap();
    let mut list = crate::settings::test_support::model_list();
    let first = list["providers"][0]["models"][0].clone();
    let models = list["providers"][0]["models"].as_array_mut().unwrap();
    for i in 0..30 {
        let mut m = first.clone();
        m["model"] = json!(format!("m{i:02}"));
        m["ref"] = json!(format!("dev/m{i:02}"));
        models.push(m);
    }
    list["pools"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "lite", "strategy": "pin",
        "models": ["relay/cline/deepseek-v4", "dev/flash"], "fixed": true}));
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
    press(&mut p, &config, KeyCode::Enter);
    let (text, _) = screen(&mut p, &config, 120, height);
    assert!(text.contains("编辑模型池"), "开了编辑窗：\n{text}");
    (p, config)
}

/// 2026-11-14 主人报：要取消一个成员得在整张模型列表里找它。已选的照池里的先后排在前，「已选 N」「未选」两行暗字隔开。
#[test]
fn chosen_members_come_first_in_the_pool_form() {
    let (mut p, config) = crowded_pool_form(60);
    let (text, _) = screen(&mut p, &config, 120, 60);
    let at = |s: &str| {
        text.lines()
            .position(|l| l.contains(s))
            .unwrap_or(usize::MAX)
    };
    let order = [
        at("已选 2"),
        at("relay/cline/deepseek-v4"),
        at("dev/flash"),
        at("未选"),
        at("dev/m00"),
    ];
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "已选在前、未选在后 {order:?}：\n{text}"
    );
}

/// 同一天主人报：列表一滚，名称、调用方式、成员、搜索框都跟着滚走。挪到成员最底下，窗顶那几行还在；矮窗口里成员也露得出来。
#[test]
fn the_pool_form_keeps_its_head_while_members_scroll() {
    for height in [30, 20] {
        let (mut p, config) = crowded_pool_form(height);
        // 一路往下挪到按钮，再退一行停在最后一个成员（relay 的 glm-5v）上
        let on_row = |p: &Settings| matches!(&p.popup, Some(Popup::Form(f)) if matches!(f.focus, Focus::Row(_)));
        while on_row(&p) {
            press(&mut p, &config, KeyCode::Char('j'));
            screen(&mut p, &config, 120, height);
        }
        press(&mut p, &config, KeyCode::Char('k'));
        let (text, _) = screen(&mut p, &config, 120, height);
        assert!(
            text.contains("zhipu/glm-5v"),
            "{height} 行：挪到了最后一个成员\n{text}"
        );
        assert!(
            text.replace(' ', "").contains("[保存"),
            "{height} 行：按钮还在\n{text}"
        );
        if height >= 30 {
            for head in ["名称", "分配策略", "子代理可用", "成员", "/ 搜索"] {
                assert!(
                    text.contains(head),
                    "{height} 行：窗顶的「{head}」没被滚走\n{text}"
                );
            }
        }
        let members = text
            .lines()
            .filter(|l| l.contains("[ ]") || l.contains("] "))
            .count();
        assert!(
            members >= 3,
            "{height} 行：成员至少露三行（{members}）\n{text}"
        );
    }
}

/// 2026-11-22 主人定：上游的列表是什么样就显示什么样，核心不再列已经没有的模型，头自己比对。池里写着、那一家的列表里没有的成员
/// 接「（供应商已不提供此模型）」、标红（连那一家都没有的照旧「（不存在）」，见 `settings::forms::tests`）。
#[test]
fn a_member_the_provider_no_longer_offers_says_so() {
    let config = Config::builtin().unwrap();
    let mut list = crate::settings::test_support::model_list();
    list["providers"][1]["models"]
        .as_array_mut()
        .unwrap()
        .remove(0);
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
    let (text, _) = screen(&mut p, &config, 160, 30);
    assert!(
        text.lines()
            .any(|l| l.contains("2  relay/cline/deepseek-v4（供应商已不提供此模型）")),
        "那一家不提供了：\n{text}"
    );
    assert!(
        text.lines()
            .any(|l| l.contains("1  dev/flash") && !l.contains("（")),
        "好的成员照旧：\n{text}"
    );
}
