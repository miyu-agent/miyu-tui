//! 伪终端测试的数据根（蓝图 `tui.md`「守着它的」）：临时的数据根和工作目录，上面跑着一份照剧本回话的核心；起界面交给
//! [`Tui`]。从 `mod.rs` 拆出来（2026-10-11：过了 500 行）。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use miyu_session::testkit::Script;
use miyu_store::env::{Env, Platform};
use miyu_store::root::DataRoot;

use super::Tui;
use super::core::{Running, Tools};

/// 一个用完就删的临时数据根和工作目录，上面跑着一份照剧本回话的核心。
pub struct Home {
    dir: PathBuf,
    /// 界面的工作目录：在数据根外面。
    pub work: PathBuf,
    /// 跑着的核心：测断线、重连时停掉再起来。
    core: Running,
}

/// 个人设置里记上「第一次引导走过了」：已经有 `[ui]` 的接在它下面（TOML 一张表只能写一次），没有的另起一张。
fn welcomed(settings: &str) -> String {
    match settings.find("[ui]\n") {
        Some(at) => {
            let (head, rest) = settings.split_at(at + "[ui]\n".len());
            format!("{head}welcomed = true\n{rest}")
        }
        None => format!("{settings}\n[ui]\nwelcomed = true\n"),
    }
}

/// 照用户装软件包那样把终端自己的清单（`tui/package.toml`）装进 alice 的家：核心 S-5 起出厂不再带它，`tui.*` 配置
/// 项、终端的状态目录都照它认（README「装上终端」那一步）。程序不放进去：界面的程序不在，配置项照旧在。
fn install_the_terminal(dir: &Path) {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("tui/package.toml");
    let to = dir.join("home/alice/packages/tui");
    std::fs::create_dir_all(&to).expect("建得了包的目录");
    std::fs::copy(manifest, to.join("package.toml")).expect("装得上终端的清单");
}

impl Home {
    /// 起一份核心：管理员 alice，请求模型照 `script` 回，没有工具。
    pub fn new(script: Script) -> Home {
        Home::with_settings(script, "")
    }

    /// 同 [`Home::new`]，核心起来以前先写好 alice 的个人设置（`home/alice/settings.toml`，TOML）。都记着第一次引导走过了
    /// （`ui.welcomed`，核心 8-11 四补），不然一起来就进引导；要引导的用 [`Home::unwelcomed`]。
    pub fn with_settings(script: Script, settings: &str) -> Home {
        Home::build(script, &welcomed(settings), Tools::None)
    }

    /// 同 [`Home::with_settings`]，但没走过第一次引导：一起来就进引导（蓝图「第一次打开的引导」第 1 条）。
    pub fn unwelcomed(script: Script, settings: &str) -> Home {
        Home::build(script, settings, Tools::None)
    }

    /// 同 [`Home::with_settings`]，另外装上基础系统的工具（读写文件、跑命令这些）：她调了要确认的，核心来问。
    pub fn with_tools(script: Script, settings: &str) -> Home {
        Home::build(script, &welcomed(settings), Tools::Shipped)
    }

    /// 同 [`Home::with_settings`]，核心只登记这一件工具（带显示名的假工具这类，`miyu_tool::testkit::Fake`）。
    pub fn with_tool(script: Script, settings: &str, tool: Arc<dyn miyu_tool::Tool>) -> Home {
        Home::build(script, &welcomed(settings), Tools::Only(vec![tool]))
    }

    fn build(script: Script, settings: &str, tools: Tools) -> Home {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("miyu-tui-{}-{n}", std::process::id()));
        let work = std::env::temp_dir().join(format!("miyu-tui-work-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&work).expect("建得了工作目录");
        let work = std::fs::canonicalize(&work).expect("在");
        let env = Env {
            platform: Platform::current(),
            miyu_home: Some(dir.clone().into_os_string()),
            home: None,
            xdg_cache_home: None,
            local_app_data: None,
            miyu_resources: None,
            exe: None,
        };
        let root = DataRoot::locate(&env).expect("MIYU_HOME 是绝对路径");
        root.prepare().expect("临时目录里建得了骨架");
        install_the_terminal(&dir);
        if !settings.is_empty() {
            let file = dir.join("home/alice/settings.toml");
            std::fs::create_dir_all(file.parent().expect("有上一层")).expect("建得了");
            std::fs::write(&file, settings).expect("写得进去");
        }
        let core = Running::start(root, script, tools);
        Home { dir, work, core }
    }

    /// 停掉核心：连着的界面断线（review B 批）。
    pub fn stop_core(&self) {
        self.core.stop();
    }

    /// 冻住核心：什么都不收发，界面这时发的请求停在路上；接着 [`Home::stop_core`] 就是请求在途时断了线。
    pub fn freeze_core(&self) {
        self.core.freeze();
    }

    /// 停掉的核心再起来：界面重连上来。
    pub fn restart_core(&self) {
        self.core.restart();
    }

    /// 停掉核心，换成没有工具，再起来（装了的工具卸掉以后的样子）。
    pub fn restart_core_without_tools(&self) {
        self.core.restart_with(Tools::None);
    }

    /// 核心的数据根。
    pub fn root(&self) -> &Path {
        &self.dir
    }

    /// 在 alice 的家目录里放一个人格（核心 P-1 上）：`persona.toml` 写 `toml`，人设写 `prompt`（`None` 不写）。
    pub fn persona(&self, id: &str, toml: &str, prompt: Option<&str>) {
        let dir = self.dir.join("home/alice/personas").join(id);
        std::fs::create_dir_all(dir.join("prompts")).expect("建得了");
        std::fs::write(dir.join("persona.toml"), toml).expect("写得进去");
        if let Some(prompt) = prompt {
            std::fs::write(dir.join("prompts/persona.md"), prompt).expect("写得进去");
        }
    }

    /// alice 的个人设置现在写着什么（没有的是空的）。
    pub fn settings(&self) -> String {
        std::fs::read_to_string(self.dir.join("home/alice/settings.toml")).unwrap_or_default()
    }

    /// 在伪终端里起界面，连这份核心；`lang` 是系统语言（`LANG`）。
    pub fn tui(&self, lang: &str) -> Tui {
        Tui::spawn(&self.dir, &self.work, lang, &[], &[])
    }

    /// 同 [`Home::tui`]，指定启动参数。
    pub fn tui_args(&self, lang: &str, args: &[&str]) -> Tui {
        Tui::spawn(&self.dir, &self.work, lang, &[], args)
    }

    /// 同 [`Home::tui_args`]，另外带几个环境变量。
    pub fn tui_args_with(&self, lang: &str, args: &[&str], env: &[(&str, &str)]) -> Tui {
        Tui::spawn(&self.dir, &self.work, lang, env, args)
    }

    /// 同 [`Home::tui`]，终端开 `cols` 列宽（看侧边栏要够 `layout.json` 的 `sidebar_from`）。
    pub fn tui_wide(&self, lang: &str, cols: u16) -> Tui {
        self.tui_wide_in(lang, cols, &self.work)
    }

    /// 同 [`Home::tui_wide`]，界面起在 `work` 这个目录。
    pub fn tui_wide_in(&self, lang: &str, cols: u16, work: &Path) -> Tui {
        Tui::program_sized(
            (&self.dir, work),
            lang,
            &[],
            &[],
            env!("CARGO_BIN_EXE_miyu-tui"),
            cols,
        )
    }

    /// 同 [`Home::tui_wide`]，另外带几个环境变量。
    pub fn tui_wide_with(&self, lang: &str, cols: u16, env: &[(&str, &str)]) -> Tui {
        Tui::program_sized(
            (&self.dir, &self.work),
            lang,
            env,
            &[],
            env!("CARGO_BIN_EXE_miyu-tui"),
            cols,
        )
    }

    /// 同 [`Home::tui`]，另外带几个环境变量。
    pub fn tui_with(&self, lang: &str, env: &[(&str, &str)]) -> Tui {
        Tui::spawn(&self.dir, &self.work, lang, env, &[])
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.work));
        drop(std::fs::remove_dir_all(&self.dir));
    }
}
