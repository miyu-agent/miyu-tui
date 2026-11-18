//! 会话列表的先后（2026-11-16 项目主人）：照人最后一次发话（核心 SX-1 的 `last_spoken`）从新到旧，只切进去不挪，人在那个
//! 会话里亲手发了一句话才挪到顶上。循环、定时任务、回报叫醒、续跑开的那几轮只动 `last_active`，不算。
//! 还没人说过话的照开的时刻（编号是 UUIDv7，前 48 位是开的那一毫秒）；不照 `last_active`：没人理的、开着循环的会话每跑一
//! 轮就会跳到最上面。侧边栏「最近会话」、`/sessions` 都照它，和网页的左栏一个先后。

use std::cmp::Reverse;

use jiff::Timestamp;

use super::SessionInfo;

/// 排先后用的时刻：人最后一次发话，没人说过话的是开的时刻，两样都没有的是 `None`（排在最后）。
pub fn order_key(info: &SessionInfo) -> Option<Timestamp> {
    info.last_spoken.or_else(|| opened(&info.session))
}

/// 照 [`order_key`] 从新到旧排；一样的照原来的先后。
pub fn sort(list: &mut [SessionInfo]) {
    list.sort_by_key(|s| Reverse(order_key(s)));
}

/// UUIDv7 编号里开的时刻：前 12 位十六进制是 Unix 毫秒。不是 UUIDv7 的是 `None`。
fn opened(session: &str) -> Option<Timestamp> {
    let bytes = session.as_bytes();
    let dashes = [8, 13, 18, 23]
        .iter()
        .all(|&at| bytes.get(at) == Some(&b'-'));
    if bytes.len() != 36 || !dashes || bytes[14] != b'7' {
        return None;
    }
    let hex = format!("{}{}", session.get(..8)?, session.get(9..13)?);
    let ms = i64::from_str_radix(&hex, 16).ok()?;
    Timestamp::from_millisecond(ms).ok()
}

#[cfg(test)]
mod tests {
    use super::opened;

    #[test]
    fn the_opening_time_comes_from_a_v7_id_only() {
        let at = opened("019a8b2c-3d4e-7f00-8000-000000000000").unwrap();
        assert_eq!(at.as_millisecond(), 0x019a_8b2c_3d4e);
        assert!(
            opened("019a8b2c-3d4e-4f00-8000-000000000000").is_none(),
            "v4"
        );
        assert!(opened("0000-aaaaaaaa").is_none());
        assert!(opened("019a8b2c3d4e7f0080000000000000000000").is_none());
    }
}
