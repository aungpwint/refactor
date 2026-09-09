mod support;

use tempfile::tempdir;

fn setup(root: &std::path::Path) {
    support::write_tsconfig_alias(root);
    support::write_file(
        root,
        "src/features/user/columns.tsx",
        "import { ResourceTable } from 'admin/resource-kit';\nexport const C = () => null;\n",
    );
    support::write_file(
        root,
        "src/features/admin/rows.tsx",
        "import { ResourceTable } from 'admin/resource-kit';\nimport X from './X';\n",
    );
    support::write_file(
        root,
        "src/features/admin/X.tsx",
        "export default function X() { return null; }\n",
    );
    support::write_file(
        root,
        "src/components/resource-kit/index.ts",
        "export const ResourceTable = () => null;\n",
    );
}

#[test]
fn imports_scan_reports_imports() {
    let dir = tempdir().expect("tempdir");
    setup(dir.path());

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--json",
        "imports",
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
    assert_eq!(parsed["command"], "imports scan");
}

#[test]
fn imports_migrate_rewrites_import_paths() {
    let dir = tempdir().expect("tempdir");
    setup(dir.path());

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "imports",
        "migrate",
        "admin/resource-kit",
        "components/resource-kit",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let user = support::read_file(dir.path(), "src/features/user/columns.tsx");
    assert!(
        user.contains("components/resource-kit"),
        "user columns: {user}"
    );

    let rows = support::read_file(dir.path(), "src/features/admin/rows.tsx");
    assert!(rows.contains("components/resource-kit"), "rows: {rows}");
    assert!(
        rows.contains("./X"),
        "non-matching import must be untouched"
    );
}

#[test]
fn imports_migrate_does_not_touch_dry_run() {
    let dir = tempdir().expect("tempdir");
    setup(dir.path());

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "imports",
        "migrate",
        "--dry-run",
        "admin/resource-kit",
        "components/resource-kit",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let user = support::read_file(dir.path(), "src/features/user/columns.tsx");
    assert!(
        user.contains("admin/resource-kit"),
        "dry-run must not change files"
    );
}

#[test]
fn paths_migrate_rewrites_references() {
    let dir = tempdir().expect("tempdir");
    setup(dir.path());

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "paths",
        "migrate",
        "admin/resource-kit",
        "components/resource-kit",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let user = support::read_file(dir.path(), "src/features/user/columns.tsx");
    assert!(
        user.contains("components/resource-kit"),
        "paths migrate must rewrite"
    );
}

#[test]
fn references_finds_import_sites() {
    let dir = tempdir().expect("tempdir");
    setup(dir.path());

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "references",
        "admin/resource-kit",
    ])
    .output()
    .expect("output");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("columns.tsx"), "should find columns.tsx");
    assert!(stdout.contains("rows.tsx"), "should find rows.tsx");
}
