//! `packaging/pack.sh` 摆的包文件夹（蓝图 `tui.md`「在哪」打成软件源的包那一条）：同发版的 `release.yml`，带上许可证两份
//! （审计 #086：原来只摆清单和程序，本机打的包里没有 `LICENSE`、`THIRD-PARTY-LICENSES.txt`）。

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 测完删掉的临时目录。
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.0));
    }
}

fn executable(path: &Path, body: &str) {
    std::fs::write(path, body).expect("写得进临时目录");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("改得了权限");
}

#[test]
fn the_package_folder_carries_both_licenses() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = std::env::temp_dir().join(format!("miyu-tui-pack-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let _scratch = Scratch(dir.clone());
    // 冒充核心：`miyu pkg pack <文件夹> --target <系统> -o <目录>`，把文件夹里有什么记下来。
    let miyu = dir.join("miyu");
    executable(
        &miyu,
        "#!/bin/sh\n[ \"$1 $2\" = 'pkg pack' ] || exit 2\n(cd \"$3\" && find . -type f | sort) > \"$7/listing\"\n",
    );
    let bin = dir.join("miyu-tui");
    executable(&bin, "#!/bin/sh\n");
    let out = dir.join("out");
    let ran = Command::new("sh")
        .arg(repo.join("packaging/pack.sh"))
        .args(["--miyu".as_ref(), miyu.as_os_str()])
        .args(["--out".as_ref(), out.as_os_str()])
        .args(["--target", "linux-x86_64"])
        .args(["--bin".as_ref(), bin.as_os_str()])
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let listing = std::fs::read_to_string(out.join("listing")).unwrap();
    let files: Vec<&str> = listing.lines().collect();
    for want in [
        "./package.toml",
        "./bin/miyu-tui",
        "./LICENSE",
        "./THIRD-PARTY-LICENSES.txt",
    ] {
        assert!(files.contains(&want), "少了 {want}：{files:?}");
    }
}
