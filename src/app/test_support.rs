//! 界面的测试夹具：一个不连核心、不画图的界面，状态照测试摆。

use tokio::sync::mpsc;

use super::App;
use crate::config::Config;
use crate::core::Command;

/// 出厂配置的界面；发给核心的命令收在交回的那一头里。
pub fn app() -> (App, mpsc::UnboundedReceiver<Command>) {
    let config = Config::builtin().expect("出厂配置读得了");
    let (core, sent) = crate::core::detached();
    let figures = crate::figures::Figures::start(None, &config.figures, None, |_| true);
    let app = App::new(config, core, crate::human::Human::default(), figures);
    (app, sent)
}
