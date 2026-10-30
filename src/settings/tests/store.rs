//! 「软件包」三页（「配置页」第 43c 条）：先看一眼再问、`y` 才做，装几个一个个来，做完写状态行、重读；开扩展要批能力的再问；
//! `Esc` 先清筛的字、再清勾选、再回主菜单。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::texts;
use crate::core::{Progress, Refusal, Stage};
use crate::settings::pages::Section;
use crate::settings::pages::store::{Flow, Listing, Local, Remote, Tab};
use crate::settings::{Settings, Tone};

fn remote(id: &str, version: &str, installed: Option<&str>) -> Remote {
    Remote {
        id: id.into(),
        name: format!("名字{id}"),
        version: version.into(),
        installed: installed.map(str::to_string),
        ..Remote::default()
    }
}

/// 进了「软件包」：软件源三个包（`b` 装着同一版），装着两个（`im` 能升级、`memes` 没设置项）。
fn opened() -> Settings {
    let mut page = Settings {
        on_menu: false,
        ..Settings::default()
    };
    page.more.section = Some(Section::Store);
    let store = &mut page.more.store;
    store.source = Listing::Got(vec![
        remote("a", "1.0.0", None),
        remote("b", "2.0.0", Some("2.0.0")),
        remote("c", "0.1.0", None),
    ]);
    let local = |id: &str, on: bool| Local {
        id: id.into(),
        name: format!("名字{id}"),
        version: "0.8.4".into(),
        on,
        ..Local::default()
    };
    store.local = Some(vec![local("im", true), local("memes", false)]);
    store.upgrades.insert("im".into(), "0.9.0".into());
    page
}

fn press(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

/// 发出去的请求：方法、参数（不要详情的）。
fn sent(page: &mut Settings) -> Vec<(u64, &'static str, Value)> {
    page.take_asks()
        .into_iter()
        .filter(|(_, m, _)| *m != "package.info")
        .collect()
}

fn status(page: &Settings) -> Option<(String, Tone)> {
    page.status.clone()
}

#[test]
fn picked_packages_are_previewed_asked_and_installed_one_by_one() {
    let mut page = opened();
    press(&mut page, KeyCode::Tab);
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Tab);
    let picked: Vec<&str> = page.more.store.picked.iter().map(String::as_str).collect();
    assert_eq!(picked, ["a", "c"], "Tab 勾选、挪到下一行");
    sent(&mut page);
    press(&mut page, KeyCode::Enter);
    let previews = sent(&mut page);
    assert_eq!(previews.len(), 2, "勾了的一个个看一眼：{previews:?}");
    assert!(
        previews
            .iter()
            .all(|(_, m, p)| *m == "package.install" && p["preview"] == true)
    );
    for (tag, _, params) in previews {
        let got = json!({"package": params["source"], "version": "1.0.0", "download": 1024});
        page.answer(tag, Ok(got), &texts());
    }
    assert!(
        matches!(page.more.store.flow, Some(Flow::Confirm { .. })),
        "都回来了才问"
    );
    press(&mut page, KeyCode::Char('j'));
    assert!(
        matches!(page.more.store.flow, Some(Flow::Confirm { .. })),
        "问的时候别的键不收"
    );
    press(&mut page, KeyCode::Char('y'));
    let first = sent(&mut page);
    assert_eq!(first.len(), 1, "一次只装一个");
    assert_eq!(first[0].2, json!({"source": "a"}));
    page.store_progress(&Progress {
        package: "a".into(),
        stage: Stage::Download,
        done: 1,
        total: 2,
    });
    let Some(Flow::Running { progress, .. }) = &page.more.store.flow else {
        panic!("在装");
    };
    assert_eq!(progress.len(), 1, "进度记在正在装的上");
    page.answer(first[0].0, Ok(json!({"package": "a"})), &texts());
    let second = sent(&mut page);
    assert_eq!(second[0].2, json!({"source": "c"}));
    page.answer(second[0].0, Ok(json!({"package": "c"})), &texts());
    assert_eq!(
        status(&page),
        Some(("已安装 名字a、名字c".into(), Tone::Good))
    );
    assert!(page.more.store.picked.is_empty(), "装好了清掉勾选");
    let reload: Vec<&str> = sent(&mut page).iter().map(|(_, m, _)| *m).collect();
    assert_eq!(
        reload,
        [
            "package.list",
            "config.schema",
            "package.upgrades",
            "package.search"
        ],
        "做完重读"
    );
}

#[test]
fn installing_what_is_already_there_or_blocked_does_not_ask() {
    let mut page = opened();
    press(&mut page, KeyCode::Down);
    sent(&mut page);
    press(&mut page, KeyCode::Enter);
    assert!(sent(&mut page).is_empty());
    assert_eq!(status(&page), Some(("名字b 已是最新".into(), Tone::Note)));
    if let Listing::Got(list) = &mut page.more.store.source {
        list[0].blocked = Some("作者被盗号".into());
    }
    press(&mut page, KeyCode::Up);
    sent(&mut page);
    press(&mut page, KeyCode::Enter);
    assert!(sent(&mut page).is_empty());
    assert_eq!(
        status(&page),
        Some(("已拉黑：作者被盗号".into(), Tone::Bad))
    );
}

#[test]
fn removing_asks_first_and_no_does_nothing() {
    let mut page = opened();
    page.more.store.tab = Tab::Installed;
    press(&mut page, KeyCode::Char('d'));
    let preview = sent(&mut page);
    assert_eq!(preview[0].1, "package.remove");
    assert_eq!(preview[0].2, json!({"package": "im", "preview": true}));
    let got = json!({"package": "im", "version": "0.8.4", "settings": ["im.token"], "state": true});
    page.answer(preview[0].0, Ok(got), &texts());
    press(&mut page, KeyCode::Char('n'));
    assert_eq!(page.more.store.flow, None);
    assert_eq!(status(&page), Some(("已取消".into(), Tone::Note)));
    assert!(sent(&mut page).is_empty(), "不卸");
}

#[test]
fn turning_on_an_extension_that_needs_approval_asks_then_approves() {
    let mut page = opened();
    page.more.store.tab = Tab::Installed;
    page.more.store.go(1);
    press(&mut page, KeyCode::Char(' '));
    let on = sent(&mut page);
    assert_eq!(
        (on[0].1, &on[0].2),
        ("package.enable", &json!({"package": "memes"}))
    );
    let refusal = Refusal {
        reason: Some("needs_approval".into()),
        message: "needs approval".into(),
        data: json!({"capabilities": ["net.fetch"]}),
    };
    page.answer(on[0].0, Err(refusal), &texts());
    assert!(matches!(page.more.store.flow, Some(Flow::Confirm { .. })));
    press(&mut page, KeyCode::Enter);
    let approve = sent(&mut page);
    assert_eq!(
        approve[0].2,
        json!({"package": "memes", "approve": ["net.fetch"]})
    );
    page.answer(approve[0].0, Ok(json!({})), &texts());
    assert_eq!(status(&page), Some(("名字memes 已打开".into(), Tone::Good)));
}

#[test]
fn upgrading_one_needs_an_update_and_all_upgrades_everything() {
    let mut page = opened();
    page.more.store.tab = Tab::Installed;
    page.more.store.go(1);
    press(&mut page, KeyCode::Char('u'));
    assert_eq!(
        status(&page),
        Some(("名字memes 已是最新".into(), Tone::Note))
    );
    sent(&mut page);
    press(&mut page, KeyCode::Char('U'));
    let preview = sent(&mut page);
    assert_eq!(preview[0].2, json!({"preview": true}), "全部升级不写编号");
    let got =
        json!({"upgrades": [{"package": "im", "from": "0.8.4", "to": "0.9.0"}], "download": 9});
    page.answer(preview[0].0, Ok(got), &texts());
    press(&mut page, KeyCode::Char('y'));
    let real = sent(&mut page);
    assert_eq!(real[0].2, json!({"preview": false}));
    let done = json!({"upgraded": [{"package": "im", "from": "0.8.4", "to": "0.9.0"}]});
    page.answer(real[0].0, Ok(done), &texts());
    assert_eq!(status(&page), Some(("已升级 名字im".into(), Tone::Good)));
}

#[test]
fn escape_clears_the_filter_then_the_picks_then_goes_back() {
    let mut page = opened();
    press(&mut page, KeyCode::Tab);
    press(&mut page, KeyCode::Char('/'));
    press(&mut page, KeyCode::Char('c'));
    press(&mut page, KeyCode::Enter);
    assert_eq!(page.more.store.rows().len(), 1, "筛出来的留着");
    press(&mut page, KeyCode::Esc);
    assert_eq!(page.more.store.query, "");
    assert_eq!(page.more.store.picked.len(), 1);
    press(&mut page, KeyCode::Esc);
    assert!(page.more.store.picked.is_empty());
    assert_eq!(page.more.section, Some(Section::Store));
    press(&mut page, KeyCode::Esc);
    assert!(
        page.on_menu && page.more.section.is_none(),
        "都没有了才回主菜单"
    );
}

#[test]
fn a_package_without_settings_says_so_instead_of_an_empty_page() {
    let mut page = opened();
    page.more.store.tab = Tab::Installed;
    press(&mut page, KeyCode::Enter);
    assert_eq!(
        status(&page),
        Some(("这个软件包没有设置项".into(), Tone::Note))
    );
    assert_eq!(page.more.section, Some(Section::Store));
}
