//! 伪终端里的端到端测试：别的目录里的技能（核心 PT-2 外）在配置页「高级」的「技能」组里，照 `config.schema` 画：开关出厂
//! 关着，「其他技能目录」出厂带一项 `~/.agents/skills`（出厂默认不空的列表），列出来、不写个数。

mod support;

use miyu_session::testkit::Script;
use support::Home;

#[test]
fn other_skill_dirs_are_listed_under_advanced_with_the_shipped_dir() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.wait_for("运行日志");
    tui.open_menu("高级");
    tui.wait_for("读取其他目录的技能");
    // 值写在名字右边：等到出厂那一项也写出来再看。
    let screen = tui.wait_for_all(&["其他技能目录", "~/.agents/skills"]);
    let row = |word: &str| {
        screen
            .iter()
            .find(|l| l.contains(word))
            .cloned()
            .unwrap_or_default()
    };
    assert!(
        row("其他技能目录").contains("~/.agents/skills"),
        "出厂那一项照原样列出来：\n{}",
        screen.join("\n")
    );
    assert!(
        row("读取其他目录的技能").contains("关"),
        "开关出厂关着：\n{}",
        screen.join("\n")
    );
}
