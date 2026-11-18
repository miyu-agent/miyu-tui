//! 蓝图「旧版那一份」的测试：家目录、`ZDOTDIR`、`XDG_CONFIG_HOME` 都经 `env` 指到临时目录，不碰真的启动文件。
//! 这一份是 bash、zsh 的启动文件；fish 的在 `fish.rs`，出错、`status`、引导在 `trouble.rs`，记号没配上对的在 `nested.rs`。

use std::fs;
use std::path::{Path, PathBuf};

use super::super::install::{Done, block};
use super::super::shells::Shell;
use super::super::words::Words;
use super::{Said, install};

mod fish;
mod nested;
mod trouble;

/// 一个测试一个临时的家目录，用完删掉。`zdotdir` 是家目录下的相对路径（`ZDOTDIR`），没有的不设。
struct Home {
    root: PathBuf,
    zdotdir: Option<PathBuf>,
}

impl Home {
    fn new(name: &str, zdotdir: Option<&str>) -> Self {
        let root =
            std::env::temp_dir().join(format!("miyu-old-hook-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).unwrap();
        let zdotdir = zdotdir.map(|z| root.join(z));
        Self { root, zdotdir }
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    fn env(&self) -> impl Fn(&str) -> Option<String> + '_ {
        move |name: &str| match name {
            "HOME" => Some(self.root.display().to_string()),
            "XDG_CONFIG_HOME" => Some(self.root.join(".config").display().to_string()),
            "ZDOTDIR" => self.zdotdir.as_ref().map(|z| z.display().to_string()),
            _ => None,
        }
    }

    /// 装进 `shell`（同 `miyu shellhook install --shell <shell>`）。
    fn install(&self, shell: Shell) -> (Done, Vec<Said>) {
        let env = self.env();
        let target = shell.target(&env).unwrap();
        install(shell, &target, program(), &env, &words()).unwrap()
    }

    /// 家目录下所有名字里带 `miyu-old-hook` 的（备份），相对路径、排好序。
    fn backups(&self) -> Vec<String> {
        let mut found = Vec::new();
        let mut dirs = vec![self.root.clone()];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                let meta = fs::symlink_metadata(&path).unwrap();
                if meta.is_dir() {
                    dirs.push(path.clone());
                }
                let rel = plain(&path.strip_prefix(&self.root).unwrap().display().to_string());
                if rel.contains("miyu-old-hook") {
                    found.push(rel);
                }
            }
        }
        found.sort();
        found
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

fn program() -> &'static Path {
    Path::new("/x/miyu")
}

fn words() -> Words {
    Words::of("zh").unwrap()
}

/// 旧版写进启动文件的那一段（`crates/miyu-base/src/shell/startup.rs`）。
fn old(shell: &str) -> String {
    format!(
        "# >>> miyu {shell} hook >>>\n[ -r '/h/.miyu/hooks/{shell}.sh' ] && source '/h/.miyu/hooks/{shell}.sh'\n# <<< miyu {shell} hook <<<\n"
    )
}

/// `said` 里有一行（`trouble` 照给的）同时写着 `parts` 的每一样。
fn says(said: &[Said], trouble: bool, parts: &[&str]) -> bool {
    said.iter().any(|s| {
        let text = plain(&s.text);
        s.trouble == trouble && parts.iter().all(|p| text.contains(p))
    })
}

/// 路径里的分隔符一律当 `/` 比：Windows 上程序拼出来的路径 `\`、`/` 混着（`…\.config\fish/conf.d\miyu.fish`），
/// 测试里另拼的同一个路径写法不一定一样。比的是说没说到这个文件，不是分隔符。
fn plain(text: &str) -> String {
    text.replace('\\', "/")
}

fn shown(path: &Path) -> String {
    plain(&path.display().to_string())
}

#[cfg(unix)]
fn mode(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn an_old_block_in_bashrc_is_backed_up_then_replaced_in_one_write() {
    let home = Home::new("bashrc", None);
    let rc = home.at(".bashrc");
    let before = format!("export A=1\n{}alias ll='ls -l'\n", old("bash"));
    fs::write(&rc, &before).unwrap();
    #[cfg(unix)]
    set_mode(&rc, 0o640);
    let (done, said) = home.install(Shell::Bash);
    assert_eq!(done, Done::Installed);
    assert_eq!(
        fs::read_to_string(&rc).unwrap(),
        format!(
            "export A=1\nalias ll='ls -l'\n{}",
            block(Shell::Bash, program())
        ),
        "第 3 条：只去掉旧记号之间那一段连两行记号，前后用户的行逐字节不变；新的一段同一次写装上"
    );
    let bak = home.at(".bashrc.miyu-old-hook.bak");
    assert_eq!(
        fs::read(&bak).unwrap(),
        before.as_bytes(),
        "第 3 条：备份是动之前的原样"
    );
    #[cfg(unix)]
    {
        assert_eq!(mode(&bak), 0o640, "第 3 条：备份的权限照原文件");
        assert_eq!(mode(&rc), 0o640, "装在哪第 5 条：原文件的权限照原来的");
    }
    assert_eq!(said.len(), 1, "第 6 条：清了一处就一行：{said:?}");
    assert!(
        says(&said, false, &[&shown(&rc), "2–4", &shown(&bak)]),
        "第 6 条：写清哪个文件、第几行到第几行、备份在哪：{said:?}"
    );
    assert_eq!(home.backups(), [".bashrc.miyu-old-hook.bak"]);

    let (done, said) = home.install(Shell::Bash);
    assert_eq!(done, Done::Updated);
    assert!(said.is_empty(), "旧的清过了，再装一次不再说：{said:?}");
    assert_eq!(
        home.backups(),
        [".bashrc.miyu-old-hook.bak"],
        "旧的清过了，再装一次不再产生备份"
    );
}

#[test]
fn an_old_block_in_a_login_file_is_only_removed_there() {
    let home = Home::new("login", None);
    let rc = home.at(".bashrc");
    fs::write(&rc, "# mine\n").unwrap();
    let profile = home.at(".bash_profile");
    let before = format!("export P=1\n{}", old("bash"));
    fs::write(&profile, &before).unwrap();
    let dot_profile = home.at(".profile");
    let dot_before = format!("{}umask 022\n", old("bash"));
    fs::write(&dot_profile, &dot_before).unwrap();
    let (done, said) = home.install(Shell::Bash);
    assert_eq!(done, Done::Installed);
    assert_eq!(
        fs::read_to_string(&profile).unwrap(),
        "export P=1\n",
        "第 1、3 条：.bash_profile 里的旧段去掉了，别的不变，也没装上新段"
    );
    assert_eq!(fs::read_to_string(&dot_profile).unwrap(), "umask 022\n");
    assert_eq!(
        fs::read_to_string(&rc).unwrap(),
        format!("# mine\n{}", block(Shell::Bash, program())),
        "新的一段照旧只装进 .bashrc"
    );
    assert_eq!(
        fs::read(home.at(".bash_profile.miyu-old-hook.bak")).unwrap(),
        before.as_bytes()
    );
    assert_eq!(
        fs::read(home.at(".profile.miyu-old-hook.bak")).unwrap(),
        dot_before.as_bytes()
    );
    assert_eq!(
        home.backups(),
        [
            ".bash_profile.miyu-old-hook.bak",
            ".profile.miyu-old-hook.bak"
        ],
        "没有旧段的 .bashrc 不备份"
    );
    assert!(says(&said, false, &[&shown(&profile), "2–4"]), "{said:?}");
    assert!(
        says(&said, false, &[&shown(&dot_profile), "1–3"]),
        "{said:?}"
    );
}

#[test]
fn zsh_looks_at_zdotdir_and_at_home() {
    let home = Home::new("zsh-zdotdir", Some("zdot"));
    fs::create_dir_all(home.at("zdot")).unwrap();
    let mine = home.at("zdot/.zshrc");
    let theirs = home.at(".zshrc");
    fs::write(&mine, format!("z\n{}", old("zsh"))).unwrap();
    fs::write(&theirs, format!("h\n{}", old("zsh"))).unwrap();
    let (_, said) = home.install(Shell::Zsh);
    assert_eq!(
        fs::read_to_string(&mine).unwrap(),
        format!("z\n{}", block(Shell::Zsh, program()))
    );
    assert_eq!(
        fs::read_to_string(&theirs).unwrap(),
        "h\n",
        "第 1 条：设了 ZDOTDIR 的，~/.zshrc 也看；只删旧段、不装新段"
    );
    assert_eq!(
        home.backups(),
        [".zshrc.miyu-old-hook.bak", "zdot/.zshrc.miyu-old-hook.bak"]
    );
    assert_eq!(said.len(), 2, "{said:?}");
}

#[test]
fn zsh_without_zdotdir_only_looks_at_home() {
    let home = Home::new("zsh-home", None);
    fs::create_dir_all(home.at("zdot")).unwrap();
    let elsewhere = home.at("zdot/.zshrc");
    let untouched = format!("z\n{}", old("zsh"));
    fs::write(&elsewhere, &untouched).unwrap();
    let rc = home.at(".zshrc");
    fs::write(&rc, old("zsh")).unwrap();
    home.install(Shell::Zsh);
    assert_eq!(
        fs::read_to_string(&elsewhere).unwrap(),
        untouched,
        "第 1 条：没设 ZDOTDIR 的只看 ~/.zshrc"
    );
    assert_eq!(
        fs::read_to_string(&rc).unwrap(),
        block(Shell::Zsh, program())
    );
    assert_eq!(home.backups(), [".zshrc.miyu-old-hook.bak"]);
}

#[test]
fn zsh_handles_the_same_file_once_when_zdotdir_is_home() {
    let home = Home::new("zsh-same", Some("."));
    let rc = home.at(".zshrc");
    fs::write(&rc, format!("a\n{}", old("zsh"))).unwrap();
    let (_, said) = home.install(Shell::Zsh);
    assert_eq!(
        fs::read_to_string(&rc).unwrap(),
        format!("a\n{}", block(Shell::Zsh, program()))
    );
    assert_eq!(
        home.backups(),
        [".zshrc.miyu-old-hook.bak"],
        "第 2 条：同一个文件不处理两遍"
    );
    assert_eq!(said.len(), 1, "{said:?}");
}

#[cfg(unix)]
#[test]
fn zsh_handles_the_same_file_once_when_zdotdir_links_to_home() {
    let home = Home::new("zsh-linked", Some("zdot"));
    std::os::unix::fs::symlink(&home.root, home.at("zdot")).unwrap();
    let rc = home.at(".zshrc");
    fs::write(&rc, format!("a\n{}", old("zsh"))).unwrap();
    let (_, said) = home.install(Shell::Zsh);
    assert_eq!(
        fs::read_to_string(&rc).unwrap(),
        format!("a\n{}", block(Shell::Zsh, program()))
    );
    assert_eq!(said.len(), 1, "第 2 条：同一个文件不处理两遍：{said:?}");
    assert!(!home.at(".zshrc.miyu-old-hook.bak.1").exists());
}

/// 旧段在 `.bash_profile`（只删、不装）：去掉后是 `after`，备份是原样，说的行号是 `lines`。
fn strip_case(name: &str, before: &[u8], after: &[u8], lines: &[&str]) {
    let home = Home::new(name, None);
    let profile = home.at(".bash_profile");
    fs::write(&profile, before).unwrap();
    let (_, said) = home.install(Shell::Bash);
    assert_eq!(
        fs::read(&profile).unwrap(),
        after,
        "{name}：只去掉该去的，别的字节不变"
    );
    assert_eq!(
        fs::read(home.at(".bash_profile.miyu-old-hook.bak")).unwrap(),
        before,
        "{name}：备份逐字节是原样"
    );
    assert_eq!(said.len(), lines.len(), "{name}：每清一处一行：{said:?}");
    for range in lines {
        assert!(
            says(&said, false, &[&shown(&profile), range]),
            "{name}：{range}：{said:?}"
        );
    }
}

#[test]
fn crlf_files_lose_only_the_block() {
    strip_case(
        "crlf",
        b"a\r\n# >>> miyu bash hook >>>\r\nsource x\r\n# <<< miyu bash hook <<<\r\nb\r\n",
        b"a\r\nb\r\n",
        &["2–4"],
    );
}

#[test]
fn a_block_at_the_end_without_a_newline_is_removed() {
    strip_case(
        "no-newline",
        b"a\n# >>> miyu bash hook >>>\nsource x\n# <<< miyu bash hook <<<",
        b"a\n",
        &["2–4"],
    );
}

#[test]
fn marks_with_spaces_around_still_count() {
    strip_case(
        "spaces",
        b"a\n  # >>> miyu bash hook >>>\t\nsource x\n\t# <<< miyu bash hook <<<  \nb\n",
        b"a\nb\n",
        &["2–4"],
    );
}

#[test]
fn every_block_in_a_file_is_removed() {
    let old = old("bash");
    strip_case(
        "two",
        format!("a\n{old}b\n{old}c\n").as_bytes(),
        b"a\nb\nc\n",
        &["2–4", "6–8"],
    );
}

#[test]
fn bytes_that_are_not_utf8_stay_as_they_are() {
    let mut before = b"\xff\xfe a\n".to_vec();
    before.extend_from_slice(old("bash").as_bytes());
    before.extend_from_slice(b"\xc3\x28 b\n");
    strip_case("not-utf8", &before, b"\xff\xfe a\n\xc3\x28 b\n", &["2–4"]);

    let home = Home::new("not-utf8-rc", None);
    let rc = home.at(".bashrc");
    fs::write(&rc, &before).unwrap();
    home.install(Shell::Bash);
    let mut after = b"\xff\xfe a\n\xc3\x28 b\n".to_vec();
    after.extend_from_slice(block(Shell::Bash, program()).as_bytes());
    assert_eq!(
        fs::read(&rc).unwrap(),
        after,
        "第 3 条：照字节处理，.bashrc 不是 UTF-8 的也只去掉旧段、装上新段"
    );
}

#[test]
fn a_block_without_its_end_is_left_alone() {
    let home = Home::new("unclosed", None);
    let profile = home.at(".bash_profile");
    let before = b"a\n# >>> miyu bash hook >>>\nsource x\n";
    fs::write(&profile, before).unwrap();
    let (_, said) = home.install(Shell::Bash);
    assert_eq!(
        fs::read(&profile).unwrap(),
        before,
        "第 1 条：只有开头没有结尾的不算一段，一个字节不动"
    );
    assert!(home.backups().is_empty(), "不动的不备份");
    assert!(
        says(&said, true, &[&shown(&profile), "2"]),
        "照旧提醒，写清哪个文件第几行：{said:?}"
    );
}

#[test]
fn marks_that_are_not_whole_lines_are_not_marks() {
    let home = Home::new("inline", None);
    let profile = home.at(".bash_profile");
    let before =
        b"echo \"# >>> miyu bash hook >>>\"\nsource x\necho \"# <<< miyu bash hook <<<\"\n";
    fs::write(&profile, before).unwrap();
    let (_, said) = home.install(Shell::Bash);
    assert_eq!(
        fs::read(&profile).unwrap(),
        before,
        "第 1 条：记号要独占一行"
    );
    assert!(home.backups().is_empty());
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn an_existing_backup_is_never_overwritten() {
    let home = Home::new("bak-taken", None);
    let profile = home.at(".bash_profile");
    let before = format!("a\n{}", old("bash"));
    fs::write(&profile, &before).unwrap();
    let taken = home.at(".bash_profile.miyu-old-hook.bak");
    fs::write(&taken, "keep me").unwrap();
    let (_, said) = home.install(Shell::Bash);
    assert_eq!(
        fs::read_to_string(&taken).unwrap(),
        "keep me",
        "第 3 条：已经有了的不盖"
    );
    let next = home.at(".bash_profile.miyu-old-hook.bak.1");
    assert_eq!(fs::read_to_string(&next).unwrap(), before, "往后试 .bak.1");
    assert!(says(&said, false, &[&shown(&next)]), "{said:?}");
}

#[cfg(unix)]
#[test]
fn a_symlinked_rc_file_is_backed_up_beside_the_link() {
    let home = Home::new("rc-link", None);
    fs::create_dir_all(home.at("dotfiles")).unwrap();
    let real = home.at("dotfiles/bashrc");
    let before = format!("a\n{}", old("bash"));
    fs::write(&real, &before).unwrap();
    set_mode(&real, 0o600);
    let link = home.at(".bashrc");
    std::os::unix::fs::symlink("dotfiles/bashrc", &link).unwrap();
    home.install(Shell::Bash);
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "第 3 条：链接还是链接"
    );
    assert_eq!(
        fs::read_to_string(&real).unwrap(),
        format!("a\n{}", block(Shell::Bash, program())),
        "改的是它指的文件"
    );
    let bak = home.at(".bashrc.miyu-old-hook.bak");
    assert_eq!(
        fs::read_to_string(&bak).unwrap(),
        before,
        "备份在链接所在的目录，内容是它指的文件的"
    );
    assert_eq!(mode(&bak), 0o600);
    assert_eq!(home.backups(), [".bashrc.miyu-old-hook.bak"]);
}

#[test]
fn nothing_old_means_nothing_extra() {
    let home = Home::new("clean", None);
    fs::write(home.at(".bashrc"), "a\n").unwrap();
    fs::write(home.at(".profile"), "b\n").unwrap();
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
        let (_, said) = home.install(shell);
        assert!(said.is_empty(), "{}：{said:?}", shell.name());
    }
    assert!(home.backups().is_empty(), "没有旧的不产生任何备份");
    assert_eq!(fs::read_to_string(home.at(".profile")).unwrap(), "b\n");
}

#[test]
fn installing_fish_leaves_the_bash_and_zsh_files_alone() {
    let home = Home::new("only-fish", None);
    let bash = format!("a\n{}", old("bash"));
    let zsh = format!("a\n{}", old("zsh"));
    fs::write(home.at(".bashrc"), &bash).unwrap();
    fs::write(home.at(".zshrc"), &zsh).unwrap();
    home.install(Shell::Fish);
    assert_eq!(
        fs::read_to_string(home.at(".bashrc")).unwrap(),
        bash,
        "第 2 条：装哪种 shell 只清哪种的"
    );
    assert_eq!(fs::read_to_string(home.at(".zshrc")).unwrap(), zsh);
    assert!(home.backups().is_empty());
}
