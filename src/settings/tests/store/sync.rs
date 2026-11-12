//! `r` 刷新软件源（发版前 check 57，2026-11-12 项目主人：「这个界面没法进行 miyu pkg update 之类更新软件列表数据之类的操作，加个 r
//! 快捷键」）：「下载软件」「已安装」两页认，发 `package.sync {force: true}`，刷着的时候再按不发；回来说一句、重读；拒了写原话。

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{opened, press, sent, status};
use crate::core::Refusal;
use crate::settings::Tone;
use crate::settings::pages::store::Tab;

#[test]
fn r_forces_a_sync_once_then_rereads_everything() {
    for tab in [Tab::Source, Tab::Installed] {
        let mut page = opened();
        page.more.store.tab = tab;
        sent(&mut page);
        press(&mut page, KeyCode::Char('r'));
        press(&mut page, KeyCode::Char('r'));
        let asked = sent(&mut page);
        assert_eq!(asked.len(), 1, "刷着的时候再按不发：{asked:?}");
        assert_eq!(
            (asked[0].1, &asked[0].2),
            ("package.sync", &json!({"force": true}))
        );
        assert_eq!(status(&page).map(|(_, t)| t), Some(Tone::Busy));
        let got = json!({"generated": "x", "fetched": "y", "packages": 43});
        page.answer(asked[0].0, Ok(got), &super::texts());
        let (line, tone) = status(&page).unwrap();
        assert_eq!(
            (line.as_str(), tone),
            ("已更新软件源：43 个软件包", Tone::Good)
        );
        let again: Vec<&str> = sent(&mut page).iter().map(|(_, m, _)| *m).collect();
        assert!(again.contains(&"package.upgrades"), "{again:?}");
        assert!(
            again.contains(&"package.search"),
            "读过软件源的重读：{again:?}"
        );
        assert!(!page.more.store.syncing, "刷完了能再按");
    }
}

#[test]
fn a_stale_or_refused_sync_says_so() {
    let mut page = opened();
    sent(&mut page);
    press(&mut page, KeyCode::Char('r'));
    let tag = sent(&mut page)[0].0;
    let stale = json!({"generated": "x", "fetched": "y", "packages": 43, "stale": "timed out"});
    page.answer(tag, Ok(stale), &super::texts());
    assert_eq!(
        status(&page),
        Some(("软件源连不上，用的是本机存的索引".into(), Tone::Note))
    );
    sent(&mut page);
    press(&mut page, KeyCode::Char('r'));
    let tag = sent(&mut page)[0].0;
    let refusal = Refusal {
        reason: Some("source_unreachable".into()),
        message: "连不上软件源".into(),
        data: json!({}),
    };
    page.answer(tag, Err(refusal), &super::texts());
    assert_eq!(status(&page), Some(("连不上软件源".into(), Tone::Bad)));
    assert!(!page.more.store.syncing);
}

#[test]
fn the_backstage_tab_ignores_r() {
    let mut page = opened();
    page.more.store.tab = Tab::Backstage;
    sent(&mut page);
    press(&mut page, KeyCode::Char('r'));
    assert!(sent(&mut page).iter().all(|(_, m, _)| *m != "package.sync"));
}
