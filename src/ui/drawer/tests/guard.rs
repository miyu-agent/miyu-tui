//! 包的守卫要人确认（核心 HK-1，排期第 61 条）：问题行写哪个包的守卫、要调哪件工具；守卫的话照原样写在下面（长的折行写全）；
//! 没有放行规则，只给「允许本次」「不允许」。

use serde_json::json;

use super::lines;
use crate::drawer::{Approval, Drawer};

fn guarded(reason: Option<&str>) -> Drawer {
    let mut detail = json!({"guard": "acme-guard", "tool": "write"});
    if let Some(reason) = reason {
        detail["reason"] = json!(reason);
    }
    let approval: Approval = serde_json::from_value(json!({
        "call_id": "c1", "access": "write", "detail": detail
    }))
    .expect("读得成");
    Drawer::approval(None, approval)
}

#[test]
fn a_guard_asks_with_its_package_tool_and_words() {
    let d = guarded(Some("周五不许写配置。\n要写请先找管理员。"));
    let shown = lines(&d, 60);
    assert!(
        shown
            .iter()
            .any(|l| l.contains("acme-guard 的守卫在问：要用 write")),
        "{shown:?}"
    );
    let at = |want: &str| shown.iter().position(|l| l.contains(want));
    assert!(
        at("周五不许写配置。").is_some() && at("要写请先找管理员。").is_some(),
        "{shown:?}"
    );
    assert!(
        at("周五不许写配置。") < at("允许本次"),
        "守卫的话在选项上面：{shown:?}"
    );
    assert!(at("不允许").is_some(), "{shown:?}");
    assert!(at("不再询问").is_none(), "没有放行规则不给：{shown:?}");
}

#[test]
fn a_long_reason_wraps_and_no_reason_shows_only_the_question() {
    let long = "这一步会改到生产环境的数据库，".repeat(8);
    let d = guarded(Some(&long));
    let shown = lines(&d, 40);
    let joined: String = shown.iter().map(|l| l.trim()).collect();
    assert!(joined.contains(&long), "折行写全：{shown:?}");
    let bare = lines(&guarded(None), 60);
    assert!(
        bare.iter().any(|l| l.contains("acme-guard 的守卫在问")),
        "{bare:?}"
    );
}
