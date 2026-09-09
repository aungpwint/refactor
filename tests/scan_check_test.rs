mod support;

use tempfile::tempdir;

fn setup_repo(root: &std::path::Path) {
    support::write_file(root, "src/index.ts", "export const a = 1;\n");
    support::write_file(
        root,
        "src/lib/util.ts",
        "export function util() { return 1; }\n",
    );
    support::write_file(root, "src/app.tsx", "import { a } from './index';\n");
    support::write_file(root, "assets/logo.svg", "<svg/>\n");
    support::write_file(root, "notes/readme.md", "# Hi\n");
}

#[test]
fn scan_reports_files_by_extension() {
    let dir = tempdir().expect("tempdir");
    setup_repo(dir.path());

    let out = support::run(&["--root", &dir.path().display().to_string(), "scan"])
        .output()
        .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("ts"), "should show ts extension stats");
    assert!(stdout.contains("Total"), "should show total");
}

#[test]
fn scan_json_output_is_valid() {
    let dir = tempdir().expect("tempdir");
    setup_repo(dir.path());

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--json",
        "scan",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_eq!(parsed["command"], "scan");
    assert!(parsed["files_scanned"].as_u64().unwrap() >= 4);
}

#[test]
fn check_passes_on_consistent_repo() {
    let dir = tempdir().expect("tempdir");
    support::write_file(root(&dir), "src/a.ts", "export const a = 1;\n");
    support::write_file(root(&dir), "src/b.ts", "import { a } from './a';\n");

    let out = support::run(&["--root", &dir.path().display().to_string(), "check"])
        .output()
        .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn check_fails_with_broken_import() {
    let dir = tempdir().expect("tempdir");
    support::write_file(
        root(&dir),
        "src/a.ts",
        "import { missing } from './does-not-exist';\n",
    );

    let out = support::run(&["--root", &dir.path().display().to_string(), "check"])
        .output()
        .expect("output");
    assert!(!out.status.success(), "check should fail on broken import");
    assert_eq!(out.status.code(), Some(1), "exit code should be 1");
}

fn root(dir: &tempfile::TempDir) -> &std::path::Path {
    dir.path()
}

#[test]
fn duplicates_detect_same_content() {
    let dir = tempdir().expect("tempdir");
    let content = "export const duplicate = 42;\n";
    for i in 0..3 {
        support::write_file(root(&dir), &format!("src/dup{i}.ts"), content);
    }

    let out = support::run(&["--root", &dir.path().display().to_string(), "duplicates"])
        .output()
        .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dup"), "should report duplicate groups");
}

#[test]
fn unused_reports_unreferenced_file() {
    let dir = tempdir().expect("tempdir");
    support::write_file(root(&dir), "src/cited.ts", "export const used = 1;\n");
    support::write_file(root(&dir), "src/orphan.ts", "export const lonely = 2;\n");
    support::write_file(
        root(&dir),
        "src/main.ts",
        "import { used } from './cited';\n",
    );

    let out = support::run(&["--root", &dir.path().display().to_string(), "unused"])
        .output()
        .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("orphan.ts"),
        "should flag unreferenced source file"
    );
}
