//! 直接读链接认出的时区和 jiff 按名字到 zoneinfo 里找的写出来一字不差。

use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{linked, local};

/// 几个时区：有夏令时的（南北半球各一）、半小时的、差四十五分钟的、调过夏令时半小时的、没有夏令时的。
const ZONES: [&str; 7] = [
    "Asia/Tokyo",
    "America/New_York",
    "Europe/London",
    "Australia/Sydney",
    "Asia/Kolkata",
    "Asia/Kathmandu",
    "Australia/Lord_Howe",
];

/// 一年里隔几天一个时刻（跨过夏令时的切换），再加几个很早的、很晚的。
fn moments() -> Vec<Timestamp> {
    let mut all: Vec<Timestamp> = (0..120)
        .map(|k| {
            Timestamp::from_second(1_767_225_600 + k * 3 * 86_400 + k * 3_571).expect("在范围里")
        })
        .collect();
    for s in [0, 946_684_800, 2_147_483_647, 4_102_444_800] {
        all.push(Timestamp::from_second(s).expect("在范围里"));
    }
    all
}

/// 照界面用到的写法和完整的时刻、差多少、简写、名字写一遍。
fn shown(tz: &TimeZone) -> Vec<String> {
    moments()
        .into_iter()
        .map(|at| {
            let z = at.to_zoned(tz.clone());
            format!(
                "{} | {} | {} | {}",
                z.strftime("%Y-%m-%d %H:%M:%S %z %Z"),
                z.strftime("%m-%d %H:%M"),
                z.date(),
                tz.iana_name().unwrap_or("-"),
            )
        })
        .collect()
}

#[test]
fn reading_the_link_writes_every_moment_like_the_named_lookup() {
    let dir = std::env::temp_dir().join(format!("miyu-tui-zone-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut compared = 0;
    for name in ZONES {
        let file = Path::new("/usr/share/zoneinfo").join(name);
        // 没装这一个时区的机器跳过它；一个都没有的在下面报。
        let Ok(named) = TimeZone::get(name) else {
            continue;
        };
        let link = dir.join(name.replace('/', "-"));
        std::os::unix::fs::symlink(&file, &link).unwrap();
        let read = linked(&link).unwrap_or_else(|| panic!("{name}：链接认不出"));
        assert_eq!(shown(&read), shown(&named), "{name}");
        compared += 1;
    }
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        compared > 0,
        "一个时区都没对照上：/usr/share/zoneinfo 是空的？"
    );
}

#[test]
fn what_is_not_a_link_into_zoneinfo_is_left_to_jiff() {
    let dir = std::env::temp_dir().join(format!("miyu-tui-zone-plain-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // 普通文件（不是链接）、链到别处的、链到不存在的：都不认，交给 jiff。
    let plain = dir.join("plain");
    std::fs::write(&plain, b"not a zone").unwrap();
    let elsewhere = dir.join("elsewhere");
    std::os::unix::fs::symlink(&plain, &elsewhere).unwrap();
    let missing = dir.join("missing");
    std::os::unix::fs::symlink(dir.join("zoneinfo/Nowhere/At_All"), &missing).unwrap();
    let broken = dir.join("zoneinfo");
    std::fs::create_dir_all(&broken).unwrap();
    let not_tzif = broken.join("Bad");
    std::fs::write(&not_tzif, b"not a zone").unwrap();
    let to_bad = dir.join("to-bad");
    std::os::unix::fs::symlink(&not_tzif, &to_bad).unwrap();
    for path in [&plain, &elsewhere, &missing, &to_bad] {
        assert!(linked(path).is_none(), "{}", path.display());
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_local_zone_writes_like_the_system_zone_on_this_machine() {
    // 本机：没设 `TZ` 的走读链接那一条，设了的交给 jiff；两样都得和 jiff 自己认的一样。
    assert_eq!(shown(&local()), shown(&TimeZone::system()));
}
