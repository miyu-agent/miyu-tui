//! 内置语义模型那一步（2026-10-18 项目主人定）：装了的问开不开；没装的看一眼软件源，能下的当场下、画进度、装不上给「重试」
//! 「跳过」（2026-10-19，核心 G-8）；软件源里没有、不合这台机器的往前、往回都跨过去。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::download::{Offer, Phase};
use super::{SemanticStep, installed};
use crate::core::Refusal;
use crate::oobe::asker::{Asker, Waiting};
use crate::oobe::{Move, Step};

fn schema(local: Value) -> Value {
    json!({"items": [
        {"key": "permission.level", "options": [{"value": "workspace"}]},
        {"key": "models.embedding", "options": [local, {"value": "off"}]}
    ]})
}

fn missing() -> Value {
    schema(json!({"value": "local", "available": false}))
}

fn refused(reason: &str, why: Option<&str>) -> Refusal {
    Refusal {
        reason: Some(reason.into()),
        message: format!("{reason} 原话"),
        data: why.map_or(Value::Null, |w| json!({"why": w})),
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::from(code)
}

/// 发出去的请求：方法和参数。
fn sent(asker: &mut Asker) -> Vec<(&'static str, Value)> {
    asker
        .take()
        .into_iter()
        .map(|a| (a.method, a.params))
        .collect()
}

/// 没装、看一眼回了 `preview`：停在这一步上。
fn not_installed(preview: Result<Value, Refusal>) -> (SemanticStep, Asker) {
    let mut step = SemanticStep::default();
    let mut asker = Asker::default();
    step.answer(&Waiting::SemanticSchema, Ok(missing()), true, &mut asker);
    assert_eq!(
        sent(&mut asker),
        [(
            "package.install",
            json!({"source": "embed", "preview": true})
        )],
        "没装的看一眼软件源"
    );
    step.answer(&Waiting::SemanticPreview, preview, true, &mut asker);
    (step, asker)
}

#[test]
fn the_package_counts_as_installed_unless_the_local_option_is_unavailable() {
    assert!(installed(&schema(json!({"value": "local"}))));
    assert!(installed(&schema(
        json!({"value": "local", "available": true})
    )));
    assert!(!installed(&missing()));
    assert!(!installed(
        &json!({"items": [{"key": "models.embedding", "options": [{"value": "off"}]}]})
    ));
    assert!(
        !installed(&json!({"items": []})),
        "核心旧、没有这一项的当没装"
    );
}

#[test]
fn nothing_to_download_for_this_machine_skips_the_step_both_ways() {
    for reason in ["unknown_package", "wrong_target"] {
        let mut step = SemanticStep::default();
        let mut asker = Asker::default();
        assert_eq!(
            step.skip(Step::Semantic, false),
            Step::Semantic,
            "还不知道的照常进"
        );
        step.answer(&Waiting::SemanticSchema, Ok(missing()), false, &mut asker);
        assert_eq!(
            step.skip(Step::Semantic, false),
            Step::Semantic,
            "没装、还没看到软件源的照常进"
        );
        let got = step.answer(
            &Waiting::SemanticPreview,
            Err(refused(reason, None)),
            false,
            &mut asker,
        );
        assert_eq!(got, Move::Stay);
        assert_eq!(
            step.skip(Step::Semantic, false),
            Step::Done,
            "{reason}：往前跨到好了"
        );
        assert_eq!(
            step.skip(Step::Semantic, true),
            Step::Preset,
            "往回跨到预设"
        );
        assert_eq!(step.skip(Step::Preset, false), Step::Preset, "别的步不管");
    }
    // 正停在这一步、看到下不了：往下走。
    let mut step = SemanticStep::default();
    let mut asker = Asker::default();
    step.answer(&Waiting::SemanticSchema, Ok(missing()), true, &mut asker);
    let got = step.answer(
        &Waiting::SemanticPreview,
        Err(refused("unknown_package", None)),
        true,
        &mut asker,
    );
    assert_eq!(got, Move::Next);
}

#[test]
fn an_unreachable_source_still_shows_the_step_without_a_size() {
    let (step, _) = not_installed(Err(refused("source_unreachable", Some("timed out"))));
    assert_eq!(step.offer, Some(Offer::Download(None)));
    assert_eq!(step.skip(Step::Semantic, false), Step::Semantic);
    let (step, _) = not_installed(Ok(json!({"package": "embed", "download": 59_768_832})));
    assert_eq!(step.offer, Some(Offer::Download(Some(59_768_832))));
}

#[test]
fn yes_downloads_on_the_spot_with_progress_and_moves_on_when_installed() {
    // 2026-10-19 项目主人：当场下、等它下完（embed 本来就是最后一步）。
    let (mut step, mut asker) = not_installed(Ok(json!({"download": 1000})));
    assert_eq!(step.key(key(KeyCode::Enter), &mut asker), Move::Stay);
    assert_eq!(
        sent(&mut asker),
        [("package.install", json!({"source": "embed"}))]
    );
    assert_eq!(step.phase, Phase::Installing { done: 0, total: 0 });
    step.progress("embed", 400, 1000);
    step.progress("other", 999, 1000);
    assert_eq!(
        step.phase,
        Phase::Installing {
            done: 400,
            total: 1000
        },
        "只认 embed 的进度"
    );
    // 下着的时候按键都不认，Esc 也不认。
    assert_eq!(step.key(key(KeyCode::Esc), &mut asker), Move::Stay);
    assert_eq!(step.key(key(KeyCode::Down), &mut asker), Move::Stay);
    assert!(sent(&mut asker).is_empty());
    // 装成了：没写过（默认就是「是」）的直接到好了，不写设置。
    let got = step.answer(&Waiting::SemanticInstall, Ok(json!({})), true, &mut asker);
    assert_eq!(got, Move::Next);
    assert!(sent(&mut asker).is_empty());
    assert_eq!(step.installed, Some(true));
}

#[test]
fn installing_over_a_written_off_switches_it_back_on() {
    let (mut step, mut asker) = not_installed(Ok(json!({"download": 1000})));
    step.answer(
        &Waiting::SemanticRead,
        Ok(json!({"items": {"models.embedding": {"value": "off"}}})),
        true,
        &mut asker,
    );
    step.pick.selected = 0;
    step.key(key(KeyCode::Enter), &mut asker);
    sent(&mut asker);
    let got = step.answer(&Waiting::SemanticInstall, Ok(json!({})), true, &mut asker);
    assert_eq!(got, Move::Stay);
    assert_eq!(
        sent(&mut asker),
        [(
            "config.set",
            json!({"changes": [{"key": "models.embedding", "value": "local"}]})
        )]
    );
    let got = step.answer(&Waiting::SemanticSave, Ok(json!({})), true, &mut asker);
    assert_eq!(got, Move::Next);
}

#[test]
fn a_failed_download_says_why_and_offers_retry_or_skip() {
    // 2026-10-19 项目主人定「原话 + 重试/跳过」。
    let (mut step, mut asker) = not_installed(Ok(json!({"download": 1000})));
    step.key(key(KeyCode::Enter), &mut asker);
    sent(&mut asker);
    let got = step.answer(
        &Waiting::SemanticInstall,
        Err(refused("download_failed", Some("mirror: 404"))),
        true,
        &mut asker,
    );
    assert_eq!(got, Move::Stay);
    assert_eq!(
        step.phase,
        Phase::Failed {
            said: "download_failed 原话".into(),
            why: Some("mirror: 404".into())
        }
    );
    // 重试：再发一次。
    assert_eq!(step.pick.selected, 0);
    step.key(key(KeyCode::Enter), &mut asker);
    assert_eq!(
        sent(&mut asker),
        [("package.install", json!({"source": "embed"}))]
    );
    step.answer(
        &Waiting::SemanticInstall,
        Err(refused("source_unreachable", None)),
        true,
        &mut asker,
    );
    // 跳过：不写设置，到好了。
    step.key(key(KeyCode::Down), &mut asker);
    assert_eq!(step.key(key(KeyCode::Enter), &mut asker), Move::Next);
    assert!(sent(&mut asker).is_empty(), "跳过的不写设置");
}

#[test]
fn no_on_a_missing_package_writes_off_like_before() {
    let (mut step, mut asker) = not_installed(Ok(json!({"download": 1000})));
    step.key(key(KeyCode::Down), &mut asker);
    step.key(key(KeyCode::Enter), &mut asker);
    assert_eq!(
        sent(&mut asker),
        [(
            "config.set",
            json!({"changes": [{"key": "models.embedding", "value": "off"}]})
        )],
        "选「否」不下载"
    );
}

#[test]
fn the_current_setting_is_where_the_pick_starts() {
    let mut step = SemanticStep::default();
    let mut asker = Asker::default();
    assert_eq!(step.pick.selected, 0, "没写的停在「是」");
    step.answer(
        &Waiting::SemanticRead,
        Ok(json!({"items": {"models.embedding": {"value": "off"}}})),
        true,
        &mut asker,
    );
    assert_eq!(step.pick.selected, 1, "写了 off 的停在「否」");
    // 装了的照旧只问开不开，不看软件源。
    let mut step = SemanticStep::default();
    step.answer(
        &Waiting::SemanticSchema,
        Ok(schema(json!({"value": "local"}))),
        true,
        &mut asker,
    );
    assert!(sent(&mut asker).is_empty());
    assert_eq!(step.skip(Step::Semantic, false), Step::Semantic);
}
