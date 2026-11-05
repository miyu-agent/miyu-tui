//! 卸载的弹窗（PK-5）：最下面一排按钮，不写「继续？」；配置那一行照停着的按钮写留在哪、会删掉哪；点按钮就是按下去。

use ratatui::layout::Position;
use serde_json::json;

use super::{page, press, screen};
use crate::config::Config;
use crate::settings::pages::Section;
use crate::settings::pages::store::flow::Doing;
use crate::settings::pages::store::{Flow, Tab};
use crate::settings::{Hit, Settings};

fn asking(config: &Config, with_config: bool) -> Settings {
    let mut p = page(config);
    p.on_menu = false;
    p.more.section = Some(Section::Store);
    p.more.store.tab = Tab::Installed;
    let home = std::env::home_dir().unwrap_or_default();
    let mut preview = json!({"package": "im", "version": "0.8.4"});
    if with_config {
        let path = home.join(".miyu/etc/im").display().to_string();
        preview["config"] = json!({"path": path, "kept": true});
    }
    p.more.store.flow = Some(Flow::Confirm {
        doing: Doing::Remove("im".into()),
        previews: vec![preview],
    });
    p.take_asks();
    p
}

#[test]
fn the_remove_popup_has_buttons_and_says_where_the_config_goes() {
    let config = Config::builtin().unwrap();
    let mut p = asking(&config, true);
    let (text, _) = screen(&mut p, &config, 90, 24);
    assert!(
        text.contains("[ 卸载 ]  [ 卸载并删除配置 ]  [ 取消 ]"),
        "{text}"
    );
    assert!(text.contains("配置 : 留在 ~/.miyu/etc/im"), "{text}");
    assert!(!text.contains("继续？"), "按钮代替「继续？」：\n{text}");
    assert!(text.contains("←/→ 选择"), "按键提示：\n{text}");
    press(&mut p, &config, ratatui::crossterm::event::KeyCode::Right);
    let (text, _) = screen(&mut p, &config, 90, 24);
    assert!(text.contains("配置 : 会删掉 ~/.miyu/etc/im"), "{text}");
    let without = &mut asking(&config, false);
    let (text, _) = screen(without, &config, 90, 24);
    assert!(text.contains("[ 卸载 ]  [ 取消 ]"), "{text}");
    assert!(
        !text.contains("删除配置") && !text.contains("配置 :"),
        "{text}"
    );
}

#[test]
fn clicking_a_button_presses_it() {
    let config = Config::builtin().unwrap();
    let mut p = asking(&config, true);
    screen(&mut p, &config, 90, 24);
    let rect = p
        .hits
        .iter()
        .find(|(_, hit)| *hit == Hit::Button(1))
        .map(|(r, _)| *r)
        .expect("画了按钮就能点");
    p.click(
        Position::new(rect.x + 2, rect.y),
        std::time::Duration::ZERO,
        &config.text.settings,
    );
    let sent: Vec<_> = p
        .take_asks()
        .into_iter()
        .filter(|(_, m, _)| *m == "package.remove")
        .map(|(_, _, params)| params)
        .collect();
    assert_eq!(sent, [json!({"package": "im", "purge": true})]);
}
