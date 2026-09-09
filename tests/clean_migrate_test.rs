mod support;

use tempfile::tempdir;

#[test]
fn clean_removes_temp_files() {
    let dir = tempdir().expect("tempdir");
    support::write_file(dir.path(), "src/a.ts", "export const a = 1;\n");
    support::write_file(dir.path(), "src/a.ts.tmp", "leftover\n");
    support::write_file(dir.path(), "src/b.ts.bak", "backup\n");

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "clean",
        "--temp-files",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(support::file_exists(dir.path(), "src/a.ts"), "source kept");
    assert!(
        !support::file_exists(dir.path(), "src/a.ts.tmp"),
        "tmp removed"
    );
    assert!(
        !support::file_exists(dir.path(), "src/b.ts.bak"),
        "bak removed"
    );
}

#[test]
fn clean_removes_empty_dirs() {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("empty_deps")).expect("create empty");
    support::write_file(dir.path(), "src/keep.ts", "export const x = 1;\n");

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "clean",
        "--empty-dirs",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !support::file_exists(dir.path(), "empty_deps"),
        "empty dir removed"
    );
    assert!(
        support::file_exists(dir.path(), "src/keep.ts"),
        "non-empty kept"
    );
}

#[test]
fn clean_with_no_targets_is_noop() {
    let dir = tempdir().expect("tempdir");
    support::write_file(dir.path(), "a.ts", "export const a = 1;\n");

    let out = support::run(&["--root", &dir.path().display().to_string(), "clean"])
        .output()
        .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(support::file_exists(dir.path(), "a.ts"));
}

#[test]
fn migrate_from_toml_plan() {
    let dir = tempdir().expect("tempdir");
    support::write_file(
        dir.path(),
        "src/old.ts",
        "const legacy = old + 'uses old token';\n",
    );
    support::write_file(
        dir.path(),
        "plan.toml",
        "version = 1\n\n[[operations]]\ntype = \"replace\"\nfrom = \"old\"\nto = \"new\"\n",
    );

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "migrate",
        &dir.path().join("plan.toml").display().to_string(),
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let content = support::read_file(dir.path(), "src/old.ts");
    assert!(
        !content.contains("old token"),
        "planned replace should apply: {content}"
    );
    assert!(content.contains("new token"));
}

#[test]
fn migrate_missing_plan_errors() {
    let dir = tempdir().expect("tempdir");

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "migrate",
        &dir.path().join("nope.toml").display().to_string(),
    ])
    .output()
    .expect("output");
    assert!(!out.status.success(), "missing plan should fail");
}
