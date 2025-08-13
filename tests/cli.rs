use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::env::{remove_var, set_var};
use std::process::Command;
use tempfile::TempDir;

fn isolated_config_home() -> TempDir {
    let td = TempDir::new().expect("tempdir");
    // Isolate XDG config and HOME so tests don't touch user environment
    set_var("XDG_CONFIG_HOME", td.path());
    set_var("HOME", td.path());
    td
}

#[test]
fn prints_help() {
    let _td = isolated_config_home();
    let mut cmd = Command::cargo_bin("topgrade").unwrap();
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains("Usage:"));
}

#[test]
fn dry_run_with_only_custom_commands_quick_and_safe() {
    let _td = isolated_config_home();
    set_var("TOPGRADE_SKIP_BRKC_NOTIFY", "true");
    // Ensure no self update ever runs even if feature toggled in the future
    set_var("TOPGRADE_NO_SELF_UPGRADE", "1");
    let mut cmd = Command::cargo_bin("topgrade").unwrap();
    cmd.args(["--dry-run", "--skip-notify", "--no-retry", "--only", "custom_commands"]);
    cmd.assert().success();
    // Clean up env that could affect subsequent tests
    remove_var("TOPGRADE_NO_SELF_UPGRADE");
    remove_var("TOPGRADE_SKIP_BRKC_NOTIFY");
}
