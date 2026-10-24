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

/// 照用户装软件包那样把终端自己的清单（`tui/package.toml`）装进系统那一层（`system/packages/`，核心 U23 上上起装的包都在这里，
/// 以前在机主家目录）：核心 S-5 起出厂不再带它，`tui.*` 配置
/// 项、终端的状态目录都照它认（README「装上终端」那一步）。程序不放进去：界面的程序不在，配置项照旧在。
/// `edit` 改一改清单再装（装着旧清单的样子）。
fn install_the_terminal(dir: &Path, edit: impl FnOnce(String) -> String) {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("tui/package.toml");
    let to = dir.join("system/packages/tui");
    std::fs::create_dir_all(&to).expect("建得了包的目录");
    let text = std::fs::read_to_string(manifest).expect("读得了终端的清单");
    std::fs::write(to.join("package.toml"), edit(text)).expect("装得上终端的清单");
    // 默认主题和纯 ASCII 样板两个主题包（蓝图「主题」第 4、6 条）：装终端时一起装。
    for theme in ["theme-default", "theme-ascii"] {
        let from = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("packages")
            .join(theme);
        let to = dir.join("system/packages").join(theme);
        std::fs::create_dir_all(&to).expect("建得了主题包的目录");
        for file in ["package.toml", "theme.json"] {
            std::fs::copy(from.join(file), to.join(file)).expect("装得上主题包");
        }
    }
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

    /// 同 [`Home::with_settings`]，装的终端清单先照 `edit` 改过（清单比终端旧、没重装的样子）。
    pub fn with_manifest(
        script: Script,
        settings: &str,
        edit: impl FnOnce(String) -> String,
    ) -> Home {
        Home::build_with(
            script,
            &welcomed(settings),
            Tools::None,
            (edit, |_: &Path| {}),
        )
    }

    /// 同 [`Home::with_settings`]，核心起来以前先照 `prepare` 往数据根里摆东西（装一个扩展包、写好开关这类）。
    pub fn prepared(script: Script, settings: &str, prepare: impl FnOnce(&Path)) -> Home {
        Home::build_with(
            script,
            &welcomed(settings),
            Tools::None,
            (|text| text, prepare),
        )
    }

    fn build(script: Script, settings: &str, tools: Tools) -> Home {
        Home::build_with(script, settings, tools, (|text| text, |_: &Path| {}))
    }

    fn build_with(
        script: Script,
        settings: &str,
        tools: Tools,
        (manifest, prepare): (impl FnOnce(String) -> String, impl FnOnce(&Path)),
    ) -> Home {
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
        install_the_terminal(&dir, manifest);
        // 软件源指到连不上的地方：测试不碰网，引导看一眼语义模型的包时是「连不上」（第 26b 条，2026-10-19 实测原来
        // 照默认地址去了 GitHub）。
        let system = dir.join("system/config.toml");
        std::fs::create_dir_all(system.parent().expect("有上一层")).expect("建得了");
        std::fs::write(
            &system,
            "[packages]\nsource = \"https://source.invalid/\"\n",
        )
        .expect("写得进去");
        // alice 是机主（核心 Y-12 中上）：握手说她是管理员，配置页有管理员模式（蓝图「管理员模式」）。同核心测具的 `owner()`。
        std::fs::write(
            dir.join("system/accounts.json"),
            r#"{"version":1,"accounts":[{"id":"alice","username":"alice","password":"$argon2id$v=19$m=19456,t=2,p=1$placeholder","changed":"2026-10-23T00:00:00.000Z","owner":true}]}"#,
        )
        .expect("写得进去");
        if !settings.is_empty() {
            let file = dir.join("home/alice/settings.toml");
            std::fs::create_dir_all(file.parent().expect("有上一层")).expect("建得了");
            std::fs::write(&file, settings).expect("写得进去");
        }
        prepare(&dir);
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
