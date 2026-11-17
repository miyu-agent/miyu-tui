//! `/model` 在还没说过话的终端里（2026-11-17 项目主人本机验收第 19 条：原来只写「先说一句再用这个命令」）。

use super::ModelPick;
use crate::config::Config;
use crate::core::{Choice, ChoiceState, Command, Update, detached};
use crate::shellhook::say::look::Look;
use crate::shellhook::say::pick::{Picked, Picker};
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::Words;

fn flash() -> Choice {
    Choice {
        reference: "magpie/deepseek-v4.1-flash".into(),
        name: "DeepSeek V4.1 Flash".into(),
        detail: String::new(),
        state: ChoiceState::Ok,
    }
}

#[test]
fn a_terminal_that_has_not_talked_yet_can_still_pick_the_model_for_its_first_session() {
    let config = Config::builtin().unwrap();
    let words = Words::of("zh").unwrap();
    let mut pick = ModelPick::new(Some("magpie/deepseek-v4.1-flash"), &words);
    assert!(!pick.needs_session(), "列表、换默认用不着终端会话");
    let mut turn = Turn::new(String::new(), None, Look::new(None));
    // 只问不开、核心回 `session: null`：这个终端还没说过话。
    turn.terminal.only_ask = true;
    turn.terminal
        .answered(Some(&serde_json::json!({"session": null})));
    assert!(turn.terminal.none);
    pick.core(&Update::Choices(vec![flash()]), &turn);
    let (core, mut sent) = detached();
    let got = pick.ready(&mut turn, &core, &config);
    assert!(matches!(got, Picked::Done(Some(_))), "定了，留一行");
    let mut chat = None;
    while let Ok(command) = sent.try_recv() {
        assert!(
            !matches!(
                command,
                Command::Ask {
                    method: "session.configure",
                    ..
                }
            ),
            "没有终端会话，不改会话"
        );
        if let Command::SetChat(reference) = command {
            chat = Some(reference);
        }
    }
    assert_eq!(
        chat.as_deref(),
        Some("magpie/deepseek-v4.1-flash"),
        "换新会话的默认"
    );
}
