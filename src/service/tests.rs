//! 读 `miyu service … --format json` 印的那一行（核心 Z-1 下补的样本），后台跑完叫醒。

use std::sync::{Arc, mpsc};
use std::time::Duration;

use super::{Reply, Service, Verb, read};

#[test]
fn the_json_line_reads_into_done_or_failed() {
    let not_installed = r#"{"file":"/home/a/.config/systemd/user/miyu.service","installed":false,"manager":"systemd","on":false}"#;
    assert_eq!(
        read(Verb::Status, not_installed),
        Reply::Done {
            verb: Verb::Status,
            installed: false,
            on: Some(false),
            file: "/home/a/.config/systemd/user/miyu.service".to_string(),
        }
    );
    let autostart = r#"{"file":"/x","installed":true,"manager":"autostart","on":null}"#;
    assert!(matches!(
        read(Verb::Enable, autostart),
        Reply::Done {
            installed: true,
            on: None,
            ..
        }
    ));
    let failed =
        r#"{"error":"failed","step":"systemctl --user enable","said":"Failed to connect to bus"}"#;
    assert_eq!(
        read(Verb::Enable, failed),
        Reply::Failed {
            verb: Verb::Enable,
            code: "failed".to_string(),
            step: "systemctl --user enable".to_string(),
            said: "Failed to connect to bus".to_string(),
        }
    );
    assert!(
        matches!(read(Verb::Enable, r#"{"error":"no_home"}"#), Reply::Failed { code, .. } if code == "no_home")
    );
    // 旧核心不认 `--format`：印的是别的，当没成、原话照印的。
    assert!(
        matches!(read(Verb::Status, "unknown flag\n"), Reply::Failed { said, .. } if said == "unknown flag")
    );
}

#[test]
fn a_run_happens_in_the_background_and_wakes_the_loop() {
    // 假的核心程序：照参数印一行（测试不碰本机真的服务管理器）。
    let dir = std::env::temp_dir().join(format!("miyu-tui-service-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(unix)]
    let (fake, script) = (
        dir.join("miyu"),
        "#!/bin/sh\n[ \"$1 $2 $3 $4\" = \"service enable --format json\" ] || exit 2\necho '{\"file\":\"/f\",\"installed\":true,\"manager\":\"systemd\",\"on\":true}'\n",
    );
    // Windows 跑不了 sh 脚本：换成同样行为的批处理。
    #[cfg(windows)]
    let (fake, script) = (
        dir.join("miyu.cmd"),
        "@echo off\r\nif not \"%1 %2 %3 %4\"==\"service enable --format json\" exit /b 2\r\necho {\"file\":\"/f\",\"installed\":true,\"manager\":\"systemd\",\"on\":true}\r\n",
    );
    std::fs::write(&fake, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut service = Service {
        program: Some(fake.clone()),
        ..Service::default()
    };
    let (ring, rang) = mpsc::channel();
    service.set_wake(Arc::new(move || if ring.send(()).is_err() {}));
    service.run(Verb::Enable);
    // 跑着时要的排着，跑完再起：假程序只认 enable，排着的 status 印不出 JSON，当没成。
    service.run(Verb::Status);
    rang.recv_timeout(Duration::from_secs(5))
        .expect("跑完叫醒主循环");
    let got = service.take().expect("叫醒以后拿得到");
    assert!(
        matches!(
            got,
            Reply::Done {
                verb: Verb::Enable,
                installed: true,
                on: Some(true),
                ..
            }
        ),
        "{got:?}"
    );
    assert!(service.busy, "排着的接着起");
    rang.recv_timeout(Duration::from_secs(5))
        .expect("排着的也叫醒");
    assert!(matches!(
        service.take(),
        Some(Reply::Failed {
            verb: Verb::Status,
            ..
        })
    ));
    assert!(!service.busy);
    // 程序不在：没成，原话是系统的。
    service.program = Some(dir.join("nope"));
    service.run(Verb::Status);
    rang.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(service.take(), Some(Reply::Failed { .. })));
    std::fs::remove_dir_all(&dir).unwrap();
}
