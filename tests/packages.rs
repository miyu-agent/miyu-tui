//! 伪终端里的端到端测试：她用 `package` 工具装包时问人的抽屉（蓝图 `tui.md`「确认和提问的抽屉」第 3 条，核心 F-10 下）接真的
//! 核心。

mod support;

use std::path::PathBuf;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 在临时目录里放一个只带一个技能的包 `weather`，交回它的绝对路径（换成真实路径：macOS 的临时目录是链接）。
fn bundle(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("miyu-tui-pkg-{}-{tag}", std::process::id()));
    let dir = root.join("weather");
    let skill = dir.join("skills/notes");
    std::fs::create_dir_all(&skill).expect("建得了目录");
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nversion = \"1.2.0\"\nname = { en = \"Weather\" }\n",
    )
    .expect("写得进去");
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: notes\ndescription: Take notes in a fixed format.\n---\nWrite notes.\n",
    )
    .expect("写得进去");
    std::fs::canonicalize(dir).expect("在")
}

/// 2026-10-12 项目主人定：照 `miyu pkg install` 列清楚——问题行是装哪个包、哪一版，下面写包含什么、多大；允许本次就装上。
/// 核心 BS-1 中起装卸包不在她的工具面上：她在命令里跑 `miyu pkg install`，经通道调 `package`，问人的是那一次嵌套调用。
#[test]
fn installing_a_package_asks_with_what_it_carries() {
    let dir = bundle("install");
    let install = serde_json::json!({"install": dir.display().to_string()}).to_string();
    let args = serde_json::json!({"commands": [["package", install]]}).to_string();
    let home = Home::with_command_shell(
        Script::new([
            Play::Calls(vec![("shell".into(), args)]),
            Play::Says("装好了。"),
        ]),
        "",
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("装个天气包");
    tui.wait_for("安装 weather 1.2.0");
    tui.wait_for("包含：技能 notes");
    tui.wait_for("大小：");
    tui.key(b"\r");
    tui.wait_for("装好了。");
    assert!(
        home.root()
            .join("system/packages/weather/package.toml")
            .exists(),
        "允许本次就装上了"
    );
    std::fs::remove_dir_all(dir.parent().expect("有上一层")).expect("删得掉测试建的目录");
}
