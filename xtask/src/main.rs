//! 终端界面仓库的杂事（发进官方源前补的，照 miyu-im 的 `xtask`）：
//!
//! - `cargo xtask notices`：照发布的四个平台的依赖图重新生成第三方许可证清单 `THIRD-PARTY-LICENSES.txt`（[`notices`]），随发行包发。
//! - `cargo xtask licenses`：查第三方依赖的许可证都能和 AGPL-3.0-or-later 合在一起发（[`licenses`]），清单是最新的；CI 跑它。

mod licenses;
mod notices;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let root = root();
    match std::env::args().nth(1).as_deref() {
        Some("notices") => match notices::write(&root, &cargo) {
            Ok(()) => {
                println!("写好了 {}", notices::PATH);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        Some("licenses") => {
            let mut problems = licenses::check(&root, &cargo);
            problems.extend(notices::check(&root, &cargo));
            if problems.is_empty() {
                println!("许可证：通过");
                ExitCode::SUCCESS
            } else {
                for problem in &problems {
                    eprintln!("{problem}");
                }
                ExitCode::FAILURE
            }
        }
        _ => {
            eprintln!("用法：cargo xtask notices | cargo xtask licenses");
            ExitCode::from(2)
        }
    }
}

/// 仓库的根目录。
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask 在仓库根目录下的 xtask/ 里，一定有上一级目录")
        .to_path_buf()
}
