//! 「MCP」那一页（蓝图「配置页」第 43b 条，2026-10-22 项目主人定）：一个服务一行，名字、状态、几件工具；网址的服务要浏览器
//! 登录的（核心 X-42 上：`state` 是 `stopped`、`reason` 是 `needs_login`）那一行右边一个「登录」，`Enter` 或点它走
//! `mcp.login`，回来的授权页交给 App 用系统的打开方式开（`take_opening`）。人在浏览器里点了同意，核心推 `mcp.changed`，
//! 重读列表，那一行变成在跑。登录过的（核心 X-42 下再补：`mcp.list` 那一项带 `logged_in: true`）那一行右边换成「登出」，
//! 走 `mcp.logout`，服务重起、停回要登录，核心推 `mcp.changed` 再重读（2026-10-22 项目主人验收时问「要怎么登出」）。
//! 先只做列表、登录、登出，加、改、删服务以后再做。
//!
//! 服务两处合成一份（同网页的 MCP 页）：系统配置里写的（`mcp.list`）在前，再是包里带的（`package.list` 每个包的
//! `mcp`），系统配置里同名的盖住包里的，包里那一个不再列。

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Value, json};

use super::More;
use crate::core::Refusal;
use crate::settings::{Settings, Texts, Tone, Waiting};

/// 这一页的字（`text/zh.json` 的 `settings.more.mcp`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpTexts {
    /// 主菜单那一项和它底下那一行。
    pub entry: [String; 2],
    /// 状态：照 `state` 写，要登录的是 `needs_login`。
    pub states: HashMap<String, String>,
    /// 在跑的接着写几件工具：`{n}`。
    pub tools: String,
    /// 要登录的那一行右边的按钮。
    pub login: String,
    /// 登录过的那一行右边的按钮。
    pub logout: String,
    /// 正在登出：`{name}`。
    pub leaving: String,
    /// 登出了：`{name}`。
    pub left: String,
    /// 没登录过（`not_logged_in`）：`{name}`。
    pub not_logged_in: String,
    /// 正在要授权页：`{name}`。
    pub asking: String,
    /// 授权页交给浏览器了：`{name}`。
    pub opened: String,
    /// 打不开浏览器：`{url}`。
    pub no_browser: String,
    /// 服务没有客户端、授权服务器又不认动态注册（`oauth_unregistered`）。
    pub unregistered: String,
    /// 登录没成（`oauth_failed`）：`{why}`。
    pub failed: String,
    /// 按键提示。
    pub hints: Vec<[String; 2]>,
}

/// 列表里的一个服务。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    /// 服务名。
    pub name: String,
    /// `off`、`starting`、`running`、`waiting`、`stopped`。
    pub state: String,
    /// 停下的为什么（`needs_login`、`failed_repeatedly`……）。
    pub reason: Option<String>,
    /// 这时在工具目录里的几件；还不知道的是 `None`。
    pub tools: Option<usize>,
    /// 存着浏览器登录换来的令牌（`logged_in: true`；没有这一格的是没登录过）。
    pub logged_in: bool,
}

impl Server {
    /// 要浏览器登录。
    pub fn needs_login(&self) -> bool {
        self.state == "stopped" && self.reason.as_deref() == Some("needs_login")
    }
}

/// 一格（`mcp.list` 的一项、`package.list` 一个包的 `mcp` 的一项）。
fn server(v: &Value) -> Option<Server> {
    Some(Server {
        name: v["name"].as_str()?.to_string(),
        state: v["state"].as_str().unwrap_or_default().to_string(),
        reason: v["reason"].as_str().map(str::to_string),
        tools: v["tools"].as_array().map(Vec::len),
        logged_in: v["logged_in"] == true,
    })
}

/// 读 `mcp.list` 的回应：系统配置里的服务。
pub fn home(got: &Value) -> Vec<Server> {
    let list = got["servers"].as_array().into_iter().flatten();
    list.filter_map(server).collect()
}

/// 读 `package.list` 的回应：包里带的服务，照包、照包里的先后。
pub fn bundled(got: &Value) -> Vec<Server> {
    let packages = got["packages"].as_array().into_iter().flatten();
    let list = packages.flat_map(|p| p["mcp"].as_array().into_iter().flatten());
    list.filter_map(server).collect()
}

impl More {
    /// 页上列的服务：系统配置里的在前，包里带的同名的不列。
    pub fn servers(&self) -> Vec<Server> {
        let mut out = self.mcp_home.clone();
        for one in &self.mcp_bundled {
            if !out.iter().any(|s| s.name == one.name) {
                out.push(one.clone());
            }
        }
        out
    }
}

impl Settings {
    /// 工具目录变了（`package.changed`、`mcp.changed`）：重读服务（包里的跟着软件包列表来）。
    pub fn tools_changed(&mut self) {
        if !self.online {
            return;
        }
        self.ask("mcp.list", json!({}), Waiting::Mcp);
        self.ask("package.list", json!({}), Waiting::Mascots);
    }

    /// `mcp.list` 回来了：记下；核心不认的（旧核心）当没有。
    pub(in crate::settings) fn mcp_listed(&mut self, result: Result<Value, Refusal>) {
        self.more.mcp_home = result.map(|got| home(&got)).unwrap_or_default();
    }

    /// 第 `at` 个服务要登录的：去要授权页；登录过的：登出。别的行 `Enter` 不做什么。
    pub(in crate::settings) fn login(&mut self, at: usize, texts: &Texts) {
        let Some(server) = self.more.servers().into_iter().nth(at) else {
            return;
        };
        let words = &texts.more.mcp;
        let params = json!({"service": server.name});
        if server.needs_login() {
            self.say(words.asking.replace("{name}", &server.name), Tone::Busy);
            self.ask("mcp.login", params, Waiting::McpLogin(server.name));
        } else if server.logged_in {
            self.say(words.leaving.replace("{name}", &server.name), Tone::Busy);
            self.ask("mcp.logout", params, Waiting::McpLogout(server.name));
        }
    }

    /// 登出回来了：成了的说一句（列表等 `mcp.changed` 重读），没成的说为什么。
    pub(in crate::settings) fn logout_answer(
        &mut self,
        name: &str,
        result: Result<Value, Refusal>,
        texts: &Texts,
    ) {
        let words = &texts.more.mcp;
        match result {
            Ok(_) => self.say(words.left.replace("{name}", name), Tone::Note),
            Err(refusal) if refusal.reason.as_deref() == Some("not_logged_in") => {
                self.say(words.not_logged_in.replace("{name}", name), Tone::Bad);
            }
            Err(refusal) => self.say(refusal.message, Tone::Bad),
        }
    }

    /// 授权页回来了：交给 App 打开；没要到的状态行说为什么。
    pub(in crate::settings) fn login_answer(
        &mut self,
        name: &str,
        result: Result<Value, Refusal>,
        texts: &Texts,
    ) {
        let words = &texts.more.mcp;
        match result {
            Ok(got) => match got["url"].as_str() {
                Some(url) => {
                    self.opening = Some(url.to_string());
                    self.say(words.opened.replace("{name}", name), Tone::Note);
                }
                None => self.say(words.failed.replace("{why}", ""), Tone::Bad),
            },
            Err(refusal) => {
                let text = match refusal.reason.as_deref() {
                    Some("oauth_unregistered") => words.unregistered.clone(),
                    Some("oauth_failed") => {
                        let why = refusal.data["why"].as_str().unwrap_or(&refusal.message);
                        words.failed.replace("{why}", why)
                    }
                    _ => refusal.message,
                };
                self.say(text, Tone::Bad);
            }
        }
    }

    /// 要打开的授权页（App 每次交完核心的回应来取）。
    pub fn take_opening(&mut self) -> Option<String> {
        self.opening.take()
    }

    /// App 打不开浏览器：状态行把授权页的网址写出来给人点。
    pub fn open_failed(&mut self, url: &str, texts: &Texts) {
        let words = &texts.more.mcp;
        self.say(words.no_browser.replace("{url}", url), Tone::Note);
    }
}

#[cfg(test)]
mod tests;
