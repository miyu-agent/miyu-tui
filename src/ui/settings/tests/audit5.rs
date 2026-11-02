//! 发布前第五批审查（2026-11-02）：「软件包」三页里，问人批不批的那一块和软件源给的字。红测试，只断言缺陷，不改产品代码。

use serde_json::json;

use super::{page, screen};
use crate::config::Config;
use crate::settings::pages::Section;
use crate::settings::pages::store::flow::Doing;
use crate::settings::pages::store::{Flow, Listing, Local, Remote, Tab};

/// 进了「软件包」的配置页：装着一个 `memes`。
fn store_page(config: &Config) -> crate::settings::Settings {
    let mut p = page(config);
    p.on_menu = false;
    p.more.section = Some(Section::Store);
    p.more.store.local = Some(vec![Local {
        id: "memes".into(),
        name: "表情包".into(),
        version: "0.1.0".into(),
        on: false,
        ..Local::default()
    }]);
    p
}

/// T5-1：开扩展要批的能力一项一行写在预览块里，块的高度照窗口剩下的算、多的行直接不画（`ui/settings/store/mod.rs`
/// 的 `take(inner.height)`），也不说下面还有。按 `y`/`Enter` 批的是全部（`flow.rs` 的 `approve: caps`）。
/// 平常的 80×24 窗口里，第 4 项以后的能力和「继续？」都不在屏上。
#[test]
fn every_capability_being_approved_is_on_screen_in_an_ordinary_window() {
    let config = Config::builtin().unwrap();
    let mut p = store_page(&config);
    p.more.store.tab = Tab::Installed;
    let caps: Vec<String> = [
        "net.fetch",
        "fs.read",
        "fs.write",
        "process.spawn",
        "secret.read",
        "session.send",
    ]
    .map(str::to_string)
    .to_vec();
    p.more.store.flow = Some(Flow::Confirm {
        doing: Doing::Enable("memes".into(), caps.clone()),
        previews: Vec::new(),
    });
    let (text, _) = screen(&mut p, &config, 80, 24);
    let missing: Vec<&String> = caps.iter().filter(|c| !text.contains(c.as_str())).collect();
    assert!(
        missing.is_empty() && text.contains("继续？"),
        "要批的能力有 {} 项没画出来、「继续？」{}，可按 y 照样全批：{missing:?}\n{text}",
        missing.len(),
        if text.contains("继续？") {
            "在"
        } else {
            "不在"
        }
    );
}

/// T5-1（装的时候）：「需要」那一行排在「一起装的依赖」后面，依赖一长、折成几行，「需要」就被挤出预览块。
#[test]
fn the_needed_capabilities_of_an_install_are_on_screen_when_dependencies_are_long() {
    let config = Config::builtin().unwrap();
    let mut p = store_page(&config);
    let deps: Vec<Remote> = (0..12)
        .map(|i| Remote {
            id: format!("dep{i}"),
            name: format!("依赖包第{i}个"),
            version: "1.0.0".into(),
            ..Remote::default()
        })
        .collect();
    let mut list = deps.clone();
    list.push(Remote {
        id: "tarot".into(),
        name: "塔罗".into(),
        version: "1.0.0".into(),
        ..Remote::default()
    });
    p.more.store.source = Listing::Got(list);
    let with: Vec<_> = deps
        .iter()
        .map(|d| json!({"package": d.id, "version": "1.0.0"}))
        .collect();
    let preview = json!({"package": "tarot", "version": "1.0.0", "download": 1024,
        "capabilities": [{"id": "process.spawn", "name": "运行程序"}], "with": with});
    p.more.store.flow = Some(Flow::Confirm {
        doing: Doing::Install(vec!["tarot".into()]),
        previews: vec![preview],
    });
    let (text, _) = screen(&mut p, &config, 80, 24);
    assert!(
        text.contains("运行程序"),
        "要装的包需要的能力没画出来（被依赖那几行挤出预览块），按 y 照样装：\n{text}"
    );
}

/// T5-3：软件源给的名字里的 kitty 图格字 U+10EEEE（T2-4 只在 `tabs::expand` 那一条入口去掉）原样画进格子：
/// 「软件包」列表、预览块不经那一条入口。
#[test]
fn a_kitty_placeholder_in_a_package_name_is_not_drawn() {
    let config = Config::builtin().unwrap();
    let mut p = store_page(&config);
    p.more.store.source = Listing::Got(vec![Remote {
        id: "x".into(),
        name: "天气\u{10EEEE}\u{0305}\u{0305}看不见".into(),
        summary: "说明\u{E0041}\u{E0042}".into(),
        version: "1.0.0".into(),
        ..Remote::default()
    }]);
    let (text, _) = screen(&mut p, &config, 80, 24);
    let bad: Vec<String> = text
        .chars()
        .filter(|c| *c == '\u{10EEEE}' || ('\u{E0020}'..='\u{E007F}').contains(c))
        .map(|c| format!("U+{:X}", u32::from(c)))
        .collect();
    assert!(
        bad.is_empty(),
        "软件源的字里的图格字、标签字符进了画面：{bad:?}"
    );
}
