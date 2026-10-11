//! 伪终端测试里跑着的那份核心（从 `home.rs` 拆出来，2026-10-11）：照剧本回话，测试可以停掉它、再起来，好测断线和
//! 重连（review B 批）。停掉是丢掉监听器、锁和每一条连接，和核心进程退出一样；数据根照旧，再起来读回原来的会话。
//! 冻住是整个运行时卡住、什么都不收发：界面这时发的请求停在路上，接着停掉，就是「请求在途时断了线」。

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use miyu_endpoint::config::Config;
use miyu_endpoint::{Core, run};
use miyu_ipc::{Dirs, open};
use miyu_kernel::id::AccountId;
use miyu_session::testkit::Script;
use miyu_store::resources::ResourceRoot;
use miyu_store::root::DataRoot;
use miyu_tool::Catalog;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::{WAIT, resources};

/// 叫核心停、起、冻住。
enum Switch {
    Stop,
    Start,
    Freeze,
}

/// 跑在自己线程的运行时里的核心；丢掉它时线程照旧跑着，跟着测试进程走。
pub struct Running {
    switch: UnboundedSender<Switch>,
    /// 停好了、起好了各报一声。
    done: Receiver<()>,
    _thread: std::thread::JoinHandle<()>,
}

impl Running {
    /// 在 `root` 上起核心，等它在听了才交回。`tools`：照出厂清单登记工具，不然没有工具。
    pub fn start(root: DataRoot, script: Script, tools: bool) -> Running {
        let (switch, mut orders) = unbounded_channel();
        let (report, done) = channel();
        let script = Arc::new(script);
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("起得来运行时");
            runtime.block_on(async move {
                let dirs = Dirs {
                    runtime_dir: None,
                    ..Dirs::current()
                };
                loop {
                    let opened = open(&root, &dirs).expect("起得来");
                    let core = Arc::new(build(&root, Arc::clone(&script), tools, opened.token));
                    if report.send(()).is_err() {
                        return;
                    }
                    if !serve_until_stopped(run(opened.listener, core), &mut orders, &report).await
                    {
                        return;
                    }
                    // 让被丢掉的连接收完尾再报停好了。
                    tokio::task::yield_now().await;
                    if report.send(()).is_err()
                        || !matches!(orders.recv().await, Some(Switch::Start))
                    {
                        return;
                    }
                }
            });
        });
        done.recv_timeout(WAIT).expect("核心起来了");
        Running {
            switch,
            done,
            _thread: thread,
        }
    }

    /// 停掉核心：连着的界面断线。
    pub fn stop(&self) {
        self.order(Switch::Stop, "核心停了");
    }

    /// 冻住：什么都不收发，界面发的请求停在路上；接着只能 [`Running::stop`]。
    pub fn freeze(&self) {
        self.order(Switch::Freeze, "核心冻住了");
    }

    /// 再起来：界面重连上来。
    pub fn restart(&self) {
        self.order(Switch::Start, "核心又起来了");
    }

    fn order(&self, order: Switch, what: &str) {
        if self.switch.send(order).is_err() {
            panic!("核心的线程没了");
        }
        self.done.recv_timeout(WAIT).expect(what);
    }
}

/// 跑着 `serving`，等到叫停（交回 `true`）；交回时丢掉它，监听器、锁、每条连接都跟着没了。叫冻住的，卡住整个运行时等叫停。
async fn serve_until_stopped(
    serving: impl Future,
    orders: &mut UnboundedReceiver<Switch>,
    report: &Sender<()>,
) -> bool {
    tokio::pin!(serving);
    tokio::select! {
        _ = &mut serving => false,
        order = orders.recv() => match order {
            Some(Switch::Stop) => true,
            Some(Switch::Freeze) => report.send(()).is_ok() && frozen(orders),
            _ => false,
        },
    }
}

/// 卡住这个线程（运行时跟着停），等叫停。
fn frozen(orders: &mut UnboundedReceiver<Switch>) -> bool {
    loop {
        match orders.try_recv() {
            Ok(Switch::Stop) => return true,
            Err(TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(10)),
            _ => return false,
        }
    }
}

/// 照核心起来时那样造一份：配置读登记的全部配置项接上软件包清单声明的（出厂的，加上装进家里的终端的 `tui.*` 这些，照
/// 发行包的样子：程序都算在），环境变量一个都不认。
fn build(root: &DataRoot, script: Arc<Script>, tools: bool, token: String) -> Core {
    let admin = AccountId::parse("alice").expect("账号合写法");
    let shipped_root = ResourceRoot::at(resources());
    let found = || miyu_store::packages::Packages::new(&shipped_root, root, &admin).read();
    let config = Config::load(
        root,
        &admin,
        None,
        miyu_core::settings::Packaged::as_released(&mut found()).all(),
        miyu_endpoint::config::Environment::of(&[]),
    );
    let catalog = if tools {
        // 照出厂的、装了的清单登记（核心 F-2 起：装了的内置包才登记工具）。
        miyu_core::tools(&shipped_root, &found()).expect("工具登记得上")
    } else {
        Catalog::default()
    };
    Core::new(
        root.clone(),
        ResourceRoot::at(resources()),
        script,
        catalog,
        None::<PathBuf>,
        admin,
        token,
    )
    .with_config(config)
}
