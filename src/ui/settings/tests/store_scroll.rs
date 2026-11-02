//! 问的时候预览块放不下（发布前第五批审查 T5-1）：露一窗、块底写下面还有几行，没滚到底算没看完；滚到底了「继续？」露出来。

use super::{page, screen};
use crate::config::Config;
use crate::settings::pages::Section;
use crate::settings::pages::store::flow::Doing;
use crate::settings::pages::store::{Flow, Tab};

#[test]
fn a_block_too_tall_for_the_window_scrolls_and_counts_as_unread_until_the_end() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    p.on_menu = false;
    p.more.section = Some(Section::Store);
    p.more.store.tab = Tab::Installed;
    let caps: Vec<String> = (1..=8).map(|i| format!("cap.{i}")).collect();
    p.more.store.flow = Some(Flow::Confirm {
        doing: Doing::Enable("memes".into(), caps),
        previews: Vec::new(),
    });
    let (text, _) = screen(&mut p, &config, 80, 18);
    assert!(text.contains("↓ 还有"), "块底写下面还有几行：\n{text}");
    assert!(!text.contains("继续？"), "{text}");
    assert!(!p.more.store.ask.seen(), "没滚到底算没看完");
    for _ in 0..10 {
        p.more.store.ask.page(true);
    }
    let (text, _) = screen(&mut p, &config, 80, 18);
    assert!(
        text.contains("继续？"),
        "滚到底：「继续？」露出来：\n{text}"
    );
    assert!(text.contains("↑ 上面还有"), "{text}");
    assert!(p.more.store.ask.seen());
}
