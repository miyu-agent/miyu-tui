//! 管理员模式（蓝图 `tui.md`「管理员模式」第 4 条，核心 Y-12 下上）：改系统那一层的请求，外壳多一格 `"scope": "system"`。
//! 哪些算照核心 `docs/reference/connection.md`「管理员模式」那一节；读的不带。

use serde_json::Value;

/// 照方法名就是改系统那一层的（参数怎么写都是）。
const SYSTEM_WRITES: [&str; 24] = [
    "account.invite",
    "account.invites",
    "account.invite_revoke",
    "account.list",
    "account.disable",
    "account.enable",
    "account.set_admin",
    "package.install",
    "package.remove",
    "package.upgrade",
    "package.enable",
    "package.disable",
    "package.clean",
    "package.sync",
    "package.restart",
    "extension.enable",
    "extension.disable",
    "extension.restart",
    "mcp.set",
    "mcp.remove",
    "mcp.login",
    "mcp.login_finish",
    "mcp.logout",
    "mcp.restart",
];

/// 这一条请求改的是系统那一层：`config.set` 看 `layer`；`secret.set`、`secret.delete` 不写 `layer` 的在管理员模式里就是
/// 系统那一份，写了 `personal` 的不算；`provider.rename` 两层一起改，算；别的照 [`SYSTEM_WRITES`]。
pub fn system_write(method: &str, params: &Value) -> bool {
    match method {
        "config.set" => params["layer"] == "system",
        "secret.set" | "secret.delete" => params["layer"] != "personal",
        "provider.rename" => true,
        _ => SYSTEM_WRITES.contains(&method),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::system_write;

    #[test]
    fn only_writes_to_the_system_layer_count() {
        assert!(system_write("config.set", &json!({"layer": "system"})));
        assert!(!system_write("config.set", &json!({"layer": "personal"})));
        assert!(system_write(
            "secret.set",
            &json!({"name": "dev", "value": "k"})
        ));
        assert!(!system_write(
            "secret.set",
            &json!({"name": "dev", "value": "k", "layer": "personal"})
        ));
        assert!(system_write(
            "secret.delete",
            &json!({"name": "dev", "layer": "system"})
        ));
        assert!(system_write("package.install", &json!({"source": "embed"})));
        assert!(system_write("mcp.logout", &json!({"server": "x"})));
        assert!(system_write(
            "provider.rename",
            &json!({"from": "a", "to": "b"})
        ));
        assert!(system_write("account.set_admin", &json!({})));
        for read in [
            "config.get",
            "package.list",
            "mcp.list",
            "secret.list",
            "model.list",
        ] {
            assert!(!system_write(read, &json!({})), "读的不带：{read}");
        }
    }
}
