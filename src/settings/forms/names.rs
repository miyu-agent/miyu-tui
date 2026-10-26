//! 配置里的编号怎么写（「路径里的名字」，核心 `miyu_config::key`）：合不合、照别的名字改成合的、和已有的错开。引导、配置页
//! 新加供应商共用（2026-10-26 从 `apply.rs`、`oobe/model/plan.rs` 挪到一起）。

/// 供应商、池的编号的写法（核心 `miyu_config::key::ID`）：小写字母打头，只有小写字母、数字、`-`、`_`，最长 32 个字符，
/// 不是 Windows 的保留名（核心拿它当文件名）。引导里起编号也照它。
pub fn name_ok(name: &str) -> bool {
    name.len() <= 32
        && !reserved(name)
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Windows 上建不了同名文件的名字（核心 `miyu_config::key` 的同一张表）。
pub fn reserved(name: &str) -> bool {
    const NAMES: [&str; 22] = [
        "con", "nul", "aux", "prn", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    NAMES.contains(&name)
}

/// 照「路径里的名字」的写法改：小写，别的字换成 `-`，不是字母开头的、是 Windows 保留名的前面加 `p-`，最多 32 个。
pub fn path_name(name: &str) -> String {
    let mut out: String = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if !out.starts_with(|c: char| c.is_ascii_lowercase()) || reserved(&out) {
        out = format!("p-{out}");
    }
    out.chars().take(32).collect()
}

/// 和 `existing` 里的错开：重了往后加 `-2`、`-3`，加了尾巴也不超过 32 个（前面截短，`path_name` 交出来的全是 ASCII）。
pub fn unique(base: String, existing: &[String]) -> String {
    if !existing.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| {
            let tail = format!("-{n}");
            format!("{}{tail}", &base[..base.len().min(32 - tail.len())])
        })
        .find(|id| !existing.contains(id))
        .unwrap_or(base)
}
