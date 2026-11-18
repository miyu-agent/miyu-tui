//! 比话里的路径用的帮手：分隔符不算。

use std::path::Path;

use super::Said;

/// `said` 里有一行（`trouble` 照给的）同时写着 `parts` 的每一样。
pub(super) fn says(said: &[Said], trouble: bool, parts: &[&str]) -> bool {
    said.iter().any(|s| {
        let text = plain(&s.text);
        s.trouble == trouble && parts.iter().all(|p| text.contains(p))
    })
}

/// 路径里的分隔符一律当 `/` 比：Windows 上程序拼出来的路径 `\`、`/` 混着（`…\.config\fish/conf.d\miyu.fish`），
/// 测试里另拼的同一个路径写法不一定一样。比的是说没说到这个文件，不是分隔符。
pub(super) fn plain(text: &str) -> String {
    text.replace('\\', "/")
}

pub(super) fn shown(path: &Path) -> String {
    plain(&path.display().to_string())
}
