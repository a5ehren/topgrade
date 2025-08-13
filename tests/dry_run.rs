use assert_cmd::prelude::*;
use std::env::set_var;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn dry_run_full_is_safe_and_exits_zero() {
    let td = TempDir::new().expect("tempdir");
    set_var("XDG_CONFIG_HOME", td.path());
    set_var("HOME", td.path());
    set_var("TOPGRADE_SKIP_BRKC_NOTIFY", "true");
    set_var("TOPGRADE_NO_SELF_UPGRADE", "1");

    let mut cmd = Command::cargo_bin("topgrade").unwrap();
    cmd.args(["--dry-run", "--skip-notify", "--no-retry"]);
    cmd.assert().success();
}
