mod support;

use tempfile::tempdir;

#[test]
fn rename_requires_yes() {
    let dir = tempdir().expect("tempdir");
    support::write_file(dir.path(), "src/old.ts", "export const a = 1;\n");

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "rename",
        "src/old.ts",
        "src/new.ts",
    ])
    .output()
    .expect("output");
    assert_eq!(
        out.status.code(),
        Some(2),
        "without --yes rename should refuse"
    );
    assert!(support::file_exists(dir.path(), "src/old.ts"));
}

#[test]
fn rename_moves_file() {
    let dir = tempdir().expect("tempdir");
    support::write_file(dir.path(), "src/old.ts", "export const a = 1;\n");
    support::write_file(
        dir.path(),
        "src/consumer.ts",
        "import { a } from './old';\n",
    );

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "rename",
        "src/old.ts",
        "src/new.ts",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(!support::file_exists(dir.path(), "src/old.ts"));
    assert!(support::file_exists(dir.path(), "src/new.ts"));

    let consumer = support::read_file(dir.path(), "src/consumer.ts");
    assert!(
        consumer.contains("./new"),
        "references must be rewritten: {consumer}"
    );
}

#[test]
fn rename_dry_run_leaves_files() {
    let dir = tempdir().expect("tempdir");
    support::write_file(dir.path(), "old.ts", "export const a = 1;\n");

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "rename",
        "--dry-run",
        "old.ts",
        "new.ts",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(support::file_exists(dir.path(), "old.ts"));
    assert!(!support::file_exists(dir.path(), "new.ts"));
}

#[test]
fn rename_updates_both_quote_styles() {
    let dir = tempdir().expect("tempdir");
    support::write_file(dir.path(), "lib/old.ts", "export const b = 2;\n");
    support::write_file(
        dir.path(),
        "lib/consumer1.ts",
        "import { b } from './old';\n",
    );
    support::write_file(
        dir.path(),
        "lib/consumer2.ts",
        "import { b } from \"./old\";\n",
    );

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "rename",
        "lib/old.ts",
        "lib/new.ts",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(support::read_file(dir.path(), "lib/consumer1.ts").contains("./new"));
    assert!(support::read_file(dir.path(), "lib/consumer2.ts").contains("./new"));
}
