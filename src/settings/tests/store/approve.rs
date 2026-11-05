//! 装、升级的时候批能力（核心 PK-3 上，设计 61）：按「是」照看一眼列的拼 `approve` 带上，装几个的一个个各带各的；旧核心的
//! 看一眼（没有 `feed`）不带。

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{opened, press, read_through, sent};
use crate::settings::pages::store::Tab;

#[test]
fn installing_approves_what_each_preview_listed() {
    let mut page = opened();
    press(&mut page, KeyCode::Tab);
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Tab);
    sent(&mut page);
    press(&mut page, KeyCode::Enter);
    let previews = sent(&mut page);
    let new = json!({"package": "a", "version": "1.0.0", "source": "official", "feed": "official",
        "capabilities": [{"id": "net", "name": "联网", "held": true}],
        "with": [{"package": "dep", "version": "0.1.0", "source": "official", "feed": "official",
                  "capabilities": [{"id": "fs.read", "name": "读文件"}]}]});
    // `c` 是旧核心的样子：`source` 是源的名字，没有 `feed`。
    let old = json!({"package": "c", "version": "0.1.0", "source": "official",
        "capabilities": [{"id": "net", "name": "联网"}]});
    page.answer(previews[0].0, Ok(new), &super::texts());
    page.answer(previews[1].0, Ok(old), &super::texts());
    page.more.store.ask = read_through(5);
    press(&mut page, KeyCode::Char('y'));
    let first = sent(&mut page);
    assert_eq!(
        first[0].2,
        json!({"source": "a", "approve": {"a": ["net"], "dep": ["fs.read"]}}),
        "批过的也带、一起装的依赖也批"
    );
    page.answer(first[0].0, Ok(json!({"package": "a"})), &super::texts());
    let second = sent(&mut page);
    assert_eq!(second[0].2, json!({"source": "c"}), "旧核心不认 approve");
}

#[test]
fn upgrading_approves_the_new_capabilities() {
    let mut page = opened();
    page.more.store.tab = Tab::Installed;
    press(&mut page, KeyCode::Char('U'));
    let preview = sent(&mut page);
    let got = json!({"upgrades": [{"package": "im", "from": "0.8.4", "to": "0.9.0",
        "source": "official", "feed": "official", "capabilities": [{"id": "net", "name": "联网"}]}],
        "with": [], "download": 9});
    page.answer(preview[0].0, Ok(got), &super::texts());
    page.more.store.ask = read_through(4);
    press(&mut page, KeyCode::Char('y'));
    let real = sent(&mut page);
    assert_eq!(
        real[0].2,
        json!({"preview": false, "approve": {"im": ["net"]}})
    );
}
