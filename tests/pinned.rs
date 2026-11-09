//! 伪终端里的端到端测试：往上翻过时正文顶上钉一行这一段对应的你说的话，点一下跳回那句话（排期第 16 条，2026-11-02 项目主人：
//! 「照 Claude Code」；挂载位 `top` 的 `prompt`）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 一段回答：每段一行，`tag` 打头好认，比一屏短（同 `tests/jump.rs`）。
fn long(tag: &str) -> &'static str {
    // 每段写长一点：比钉着的那句话长，盖不住时露得出来。
    let text: Vec<String> = (1..=10)
        .map(|i| {
            format!("{tag} 第 {i} 段。这一段写得比问的那句话长一些，好看出下面的字有没有露出来。")
        })
        .collect();
    Box::leak(text.join("\n\n").into_boxed_str())
}

/// 有这句的那几行。
fn rows_with(lines: &[String], word: &str) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(word))
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn scrolled_up_the_top_pins_the_question_and_a_click_jumps_back() {
    let home = Home::new(Script::new([
        Play::Says(long("甲")),
        Play::Says(long("乙")),
        Play::Says(long("丙")),
    ]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    for (said, last) in [
        ("第一问 alpha", "甲 第 10 段。"),
        ("第二问 beta", "乙 第 10 段。"),
        ("第三问 gamma", "丙 第 10 段。"),
    ] {
        tui.say(said);
        tui.wait_for(last);
    }
    tui.pump(Duration::from_millis(300));
    // 正文上滚轮一格一格往上，直到第二问整句滚出视口、顶上钉着它：只有正文第一行有它，下面露着乙的回答。
    let mut pinned = false;
    for _ in 0..30 {
        tui.key(b"\x1b[<64;20;10M");
        tui.pump(Duration::from_millis(80));
        let lines = tui.lines();
        // 正文区顶上空一行（`top_gap`），钉着的那一行在屏幕第 1 行；下面不是你说的话自带的那一行空竖线。
        let at = rows_with(&lines, "第二问 beta");
        if at.len() == 1
            && at[0] <= 1
            && lines[at[0] + 1].trim() != "┃"
            && lines[at[0] + 1..at[0] + 6]
                .iter()
                .any(|l| l.contains("乙 第"))
        {
            pinned = true;
            break;
        }
    }
    assert!(
        pinned,
        "往上翻到乙的回答，顶上钉着第二问：\n{}",
        tui.lines().join("\n")
    );
    // 钉着的那一行整行盖住下面的正文：句子后面不露出底下那一行剩下的字（2026-11-05 验收：钉的字后面露出半截回答）。
    let lines = tui.lines();
    let row = rows_with(&lines, "第二问 beta")[0];
    assert!(
        lines[row].trim_end().ends_with("第二问 beta"),
        "钉着的那一行后面不该有残字：{:?}",
        lines[row]
    );
    // 竖线贴最左边，竖线和字都是暗色（发版前 check 第 6 条，2026-11-09 项目主人：原来缩进照正文、竖线照权限级别的颜色、
    // 字是亮的）：同一个颜色，竖线后面空一格就是字。
    let line = &lines[row];
    let bar = line.chars().take_while(|c| *c == ' ').count();
    assert!(
        line[bar..].starts_with("┃ 第二问"),
        "竖线后面一格就是字：{line:?}"
    );
    let row16 = u16::try_from(row).expect("行号");
    let col = u16::try_from(bar).expect("列");
    assert_eq!(
        tui.foreground(row16, col),
        tui.foreground(row16, col + 2),
        "竖线和字同一个暗色"
    );
    assert_ne!(
        tui.foreground(row16, col + 2),
        vt100::Color::Default,
        "字不是默认的亮色"
    );
    // 点钉着的那一行：跳回第二问，它自己那一行顶到视口最上面（竖线那一行之下），不再钉着。
    let lines = tui.lines();
    let row = rows_with(&lines, "第二问 beta")[0];
    let col = lines[row][..lines[row].find("第二问").expect("在")]
        .chars()
        .count()
        + 1;
    tui.key(format!("\x1b[<0;{col};{0}M\x1b[<0;{col};{0}m", row + 1).as_bytes());
    let start = std::time::Instant::now();
    loop {
        tui.pump(Duration::from_millis(20));
        let lines = tui.lines();
        let at = rows_with(&lines, "第二问 beta");
        if at.len() == 1 && (1..=3).contains(&at[0]) && lines[at[0] - 1].trim() == "┃" {
            // 正文里你说的话，竖线前面照输入框空着缩进；钉着的那一行不空（紧凑版面两边都贴左边，不比）
            let own = lines[at[0] - 1].chars().take_while(|c| *c == ' ').count();
            assert!(
                own == 0 || bar < own,
                "钉着的竖线比正文里的靠左：{bar} / {own}"
            );
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "没跳回去：\n{}",
            lines.join("\n")
        );
    }
}

/// `[[tui.top]]` 藏起 `prompt`：一路往上翻，顶上不钉（「挂载位」第 4 条）。
#[test]
fn a_hidden_prompt_part_never_pins() {
    let home = Home::with_settings(
        Script::new([
            Play::Says(long("甲")),
            Play::Says(long("乙")),
            Play::Says(long("丙")),
        ]),
        "[[tui.top]]\npart = \"prompt\"\nhidden = true\n",
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    for (said, last) in [
        ("第一问 alpha", "甲 第 10 段。"),
        ("第二问 beta", "乙 第 10 段。"),
        ("第三问 gamma", "丙 第 10 段。"),
    ] {
        tui.say(said);
        tui.wait_for(last);
    }
    tui.pump(Duration::from_millis(300));
    for _ in 0..30 {
        tui.key(b"\x1b[<64;20;10M");
        tui.pump(Duration::from_millis(80));
        let lines = tui.lines();
        let at = rows_with(&lines, "第二问 beta");
        assert!(
            !(at.len() == 1 && at[0] <= 1 && lines[at[0] + 1].trim() != "┃"),
            "藏起来的不钉：\n{}",
            lines.join("\n")
        );
    }
}
