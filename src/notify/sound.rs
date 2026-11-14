//! 提示音（蓝图 `tui.md`「系统通知」第 5 条）：内置的 `wake.wav`（旧版合成的木琴音），第一次用时写到机器缓存
//! 目录的 `tui/sounds/wake.wav`，大小对不上就重写；照清单依次试放音的程序，哪个拉得起来用哪个。

use std::fs;
use std::path::{Path, PathBuf};

use super::system::spawn;

/// 内置的提示音。
const WAKE: &[u8] = include_bytes!("../../resources/sounds/wake.wav");

/// 把提示音写进 `dir`（已经在、大小对的不重写），交回文件；写不进去的是 `None`。
pub fn materialize(dir: &Path) -> Option<PathBuf> {
    let file = dir.join("wake.wav");
    let fresh = fs::metadata(&file).is_ok_and(|m| m.len() == WAKE.len() as u64);
    if !fresh {
        fs::create_dir_all(dir).ok()?;
        fs::write(&file, WAKE).ok()?;
    }
    Some(file)
}

/// 一个放音程序的参数：写了 `{file}` 的那一格换成音的文件（单引号成对写，PowerShell 单引号字串里的写法：Windows 那一条
/// 把路径写进脚本里，2026-11-15 三平台摸底），没写的接在最后。
pub fn player_args(args: &[String], file: &Path) -> Vec<String> {
    let path = file.display().to_string();
    if args.iter().any(|a| a.contains("{file}")) {
        return args
            .iter()
            .map(|a| a.replace("{file}", &path.replace('\'', "''")))
            .collect();
    }
    let mut args = args.to_vec();
    args.push(path);
    args
}

/// 放一声：照 `players` 的先后，拉得起来就算；一个都拉不起来的什么都不做。
pub fn play(players: &[Vec<String>], file: &Path) {
    for player in players {
        let Some((program, args)) = player.split_first() else {
            continue;
        };
        if spawn(program, &player_args(args, file)) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WAKE, materialize, player_args};

    /// Windows 那一条把路径写进 PowerShell 脚本：`{file}` 换成路径，单引号成对写；别的照旧接在最后（2026-11-15 三平台摸底）。
    #[test]
    fn the_file_goes_where_the_player_says() {
        let file = std::path::Path::new("/c/Users/o'brien/wake.wav");
        let own = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            player_args(&own(&["-q"]), file),
            own(&["-q", "/c/Users/o'brien/wake.wav"])
        );
        assert_eq!(
            player_args(
                &own(&[
                    "-Command",
                    "(New-Object Media.SoundPlayer '{file}').PlaySync()"
                ]),
                file
            ),
            own(&[
                "-Command",
                "(New-Object Media.SoundPlayer '/c/Users/o''brien/wake.wav').PlaySync()"
            ])
        );
    }

    #[test]
    fn the_sound_is_written_once_and_rewritten_when_it_goes_wrong() {
        let dir = std::env::temp_dir().join(format!("miyu-sound-{}", std::process::id()));
        let file = materialize(&dir).unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), WAKE);
        std::fs::write(&file, b"broken").unwrap();
        materialize(&dir).unwrap();
        assert_eq!(
            std::fs::read(&file).unwrap().len(),
            WAKE.len(),
            "大小不对就重写"
        );
        std::fs::remove_dir_all(&dir).unwrap_or_default();
    }
}
