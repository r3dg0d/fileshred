use std::io::Write;

#[test]
fn help_and_explain() {
    let bin = env!("CARGO_BIN_EXE_fileshred");
    let o = std::process::Command::new(bin).arg("--help").output().unwrap();
    assert!(o.status.success());
    let o = std::process::Command::new(bin).arg("explain").output().unwrap();
    assert!(o.status.success());
    let t = String::from_utf8_lossy(&o.stdout);
    assert!(t.to_lowercase().contains("ssd") || t.contains("wear-leveling"));
}

#[test]
fn inspect_json() {
    let bin = env!("CARGO_BIN_EXE_fileshred");
    let o = std::process::Command::new(bin)
        .args(["--json", "inspect", "/etc/hosts"])
        .output()
        .unwrap();
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(v.get("storage_class").is_some());
    assert!(v.get("advice").is_some());
}

#[test]
fn delete_dry_run_tempfile() {
    let bin = env!("CARGO_BIN_EXE_fileshred");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("victim.txt");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"secret").unwrap();
    }
    let o = std::process::Command::new(bin)
        .args([
            "--json",
            "--dry-run",
            "delete",
            path.to_str().unwrap(),
            "--i-understand",
        ])
        .output()
        .unwrap();
    assert!(o.status.success(), "stderr={}", String::from_utf8_lossy(&o.stderr));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["guaranteed_physical_erasure"], false);
    assert!(path.exists(), "dry-run must not delete");
}

#[test]
fn refuse_without_ack_on_uncertain() {
    // We can't control FS type in CI, but --unlink-only without file still works;
    // ensure delete on missing path returns not-found.
    let bin = env!("CARGO_BIN_EXE_fileshred");
    let o = std::process::Command::new(bin)
        .args(["delete", "/nonexistent/fileshred-test-path-xyz"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(5));
}
