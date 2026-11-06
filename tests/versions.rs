//! 终端本体和 ASCII 主题包的版本跟程序走（核心 VR-1，2026-11-06 主人定版本号写「年.月.这个月第几版」，四件同号；同一天定
//! ASCII 主题包和本体同号）：`Cargo.toml` 的版本、`miyu-tui/package.toml`、`packages/miyu-tui-theme-ascii/package.toml` 三处要一样。
//! 两个包在同一个标签里发，打包模板（miyu-agent/packages 的 pack.yml）要求标签去掉 v 和清单的版本一样，差一处整趟发版就停。
//! 默认主题包不发，不管它。

use std::path::Path;

/// 清单 `[package]` 下 `version = "…"` 那一行的值（只认这一种写法，清单都是这样写的）。
fn manifest_version(path: &str) -> String {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .expect("读得到清单");
    let mut section = "";
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line;
        } else if section == "[package]"
            && let Some(rest) = line.strip_prefix("version")
            && let Some(value) = rest.trim_start().strip_prefix('=')
        {
            return value.trim().trim_matches('"').to_string();
        }
    }
    panic!("{path} 的 [package] 没写 version");
}

#[test]
fn the_packages_carry_the_programs_version() {
    let program = env!("CARGO_PKG_VERSION");
    assert_eq!(manifest_version("miyu-tui/package.toml"), program);
    assert_eq!(
        manifest_version("packages/miyu-tui-theme-ascii/package.toml"),
        program
    );
}
