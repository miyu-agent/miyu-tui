//! 引导当场下语义模型时断了线（第 26b 条，2026-10-19）：在下的那一条照被拒办，写断开的原因、给「重试」「跳过」，不卡在进度条上。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{asks, find, fresh, key, texts};
use crate::oobe::Step;
use crate::oobe::semantic::download::{Offer, Phase};

#[test]
fn losing_the_core_mid_download_offers_retry_instead_of_hanging() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.semantic.offer = Some(Offer::Download(Some(1000)));
    oobe.step = Step::Semantic;
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "package.install");
    assert_eq!(params, json!({"source": "embed"}));
    assert!(oobe.semantic.installing());
    oobe.offline("核心断开了", t0, &texts);
    assert_eq!(
        oobe.semantic.phase,
        Phase::Failed {
            said: "核心断开了".into(),
            why: None
        }
    );
    assert_eq!(oobe.step, Step::Semantic, "停在这一步等人选");
}
