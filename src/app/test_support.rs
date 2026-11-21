//! 界面的测试夹具：一个不连核心、不画图的界面，状态照测试摆。

use tokio::sync::mpsc;

use super::App;
use crate::config::Config;
use crate::core::Command;

/// 出厂配置的界面；发给核心的命令收在交回的那一头里。
pub fn app() -> (App, mpsc::UnboundedReceiver<Command>) {
    let (app, sent, _) = app_with_starts();
    (app, sent)
}

/// 同 [`app`]，另交回「启动」那一头（核心是人停的时点了「启动」，`core_stopped.rs`）。
pub fn app_with_starts() -> (
    App,
    mpsc::UnboundedReceiver<Command>,
    mpsc::UnboundedReceiver<()>,
) {
    let config = Config::builtin().expect("出厂配置读得了");
    let (core, sent, starts) = crate::core::detached_with_starts();
    let figures = crate::figures::Figures::start(None, &config.figures, None, |_| true);
    let app = App::new(
        Box::new(config),
        core,
        crate::human::Human::default(),
        figures,
    );
    (app, sent, starts)
}
