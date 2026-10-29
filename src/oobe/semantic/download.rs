//! 没装语义模型的包时当场下载（蓝图 `tui.md`「第一次打开的引导」第 26b 条，核心 G-8，2026-10-19 项目主人定当场下、等它下完，
//! 装不上的照原话写、给「重试」「跳过」）：先看一眼要下多大，选「是」照软件源装，下的时候照推来的进度画条。

use serde_json::{Value, json};

use crate::core::Refusal;

/// 软件源里语义模型的包。
pub const PACKAGE: &str = "miyu-embed";

/// 看一眼的结果：这一屏出不出、写多大。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    /// 能下：要下多少字节（连不上软件源的不知道，是 `None`）。
    Download(Option<u64>),
    /// 软件源里没有、没有这台机器能用的：这一屏跨过去。
    Unavailable,
}

impl Offer {
    /// 看一眼的回应：回了的写大小；连不上的照出、不写大小；没有、不合这台机器的跨过去（别的拒绝同连不上）。
    pub fn read(result: Result<Value, Refusal>) -> Self {
        match result {
            Ok(got) => Self::Download(got["download"].as_u64()),
            Err(refusal) => match refusal.reason.as_deref() {
                Some("unknown_package" | "wrong_target") => Self::Unavailable,
                _ => Self::Download(None),
            },
        }
    }
}

/// 下载这一段到哪了。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// 还没下：两行是「是」「否」。
    Choose,
    /// 在下：下了多少、一共多少（还没推来的都是 0）。
    Installing {
        /// 下了多少字节。
        done: u64,
        /// 一共多少字节。
        total: u64,
    },
    /// 装不上：核心的原话、有的话 `data.why`；两行是「重试」「跳过」。
    Failed {
        /// 核心的原话。
        said: String,
        /// 原因（英文，核心照每一处的先后接起来的）。
        why: Option<String>,
    },
}

/// 看一眼的参数。
pub fn preview() -> Value {
    json!({"source": PACKAGE, "preview": true})
}

/// 真装的参数。
pub fn install() -> Value {
    json!({"source": PACKAGE})
}

/// 装不上：照原话记下。
pub fn failed(refusal: &Refusal) -> Phase {
    Phase::Failed {
        said: crate::settings::pages::target::said(refusal),
        why: refusal.data["why"]
            .as_str()
            .filter(|w| !w.is_empty())
            .map(str::to_string),
    }
}
