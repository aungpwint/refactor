mod support;

use tempfile::tempdir;

fn setup_repo(root: &std::path::Path) {
    support::write_file(
        root,
        "app/resources/js/admin/academic/columns.tsx",
        "import { ResourceTable } from 'admin/resource-kit';\n\nexport default function Columns() {\n  return <ResourceTable />;\n}\n",
    );
    support::write_file(
        root,
        "app/resources/js/admin/admission/columns.tsx",
        "import { ResourceTable } from 'admin/resource-kit';\n\nexport default function AdmissionColumns() {\n  return <ResourceTable />;\n}\n",
    );
    support::write_file(
        root,
        "components/resource-kit/index.ts",
        "export const ResourceTable = () => null;\n",
    );
    support::write_file(
        root,
        "app/App.tsx",
        "import { ResourceTable } from 'admin/resource-kit';\nimport Columns from './admin/academic/columns';\n",
    );
}

#[test]
fn replace_updates_all_matching_files() {
    let dir = tempdir().expect("tempdir");
    setup_repo(dir.path());

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "replace",
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

    let academic = support::read_file(dir.path(), "app/resources/js/admin/academic/columns.tsx");
    assert!(
        !academic.contains("admin/resource-kit"),
        "academic not updated: {academic}"
    );
    assert!(academic.contains("components/resource-kit"));

    let admission = support::read_file(dir.path(), "app/resources/js/admin/admission/columns.tsx");
    assert!(
        admission.contains("components/resource-kit"),
        "admission not updated"
    );

    let app = support::read_file(dir.path(), "app/App.tsx");
    assert!(app.contains("components/resource-kit"), "app not updated");
    assert!(
        app.contains("./admin/academic/columns"),
        "relative import must be unchanged"
    );
}

#[test]
fn replace_dry_run_does_not_modify() {
    let dir = tempdir().expect("tempdir");
    setup_repo(dir.path());

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "replace",
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

    let academic = support::read_file(dir.path(), "app/resources/js/admin/academic/columns.tsx");
    assert!(
        academic.contains("admin/resource-kit"),
        "dry-run must not modify"
    );
    assert!(!academic.contains("components/resource-kit"));
}

#[test]
fn replace_respects_bom_and_crlf() {
    let dir = tempdir().expect("tempdir");

    let bom: &[u8] = &[0xEF, 0xBB, 0xBF];
    let body = b"import a from 'admin/resource-kit';\r\nconst x = 1;\r\n";
    let mut content = bom.to_vec();
    content.extend_from_slice(body);
    support::write_file_bytes(dir.path(), "bom.ts", &content);

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "replace",
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

    let bytes = std::fs::read(dir.path().join("bom.ts")).expect("read back");
    assert_eq!(&bytes[0..3], &[0xEF, 0xBB, 0xBF], "BOM must be preserved");
    let text = String::from_utf8(bytes[3..].to_vec()).expect("utf8");
    assert!(
        text.contains("components/resource-kit"),
        "replacement applied"
    );
    assert!(text.contains("\r\n"), "CRLF preserved: {text:?}");
}

#[test]
fn replace_skips_binary_files() {
    let dir = tempdir().expect("tempdir");
    support::write_file_bytes(dir.path(), "binary.ts", b"not text with null:\0\x01\x02");
    support::write_file(
        dir.path(),
        "text.ts",
        "import x from 'admin/resource-kit';\n",
    );

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "replace",
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

    let binary = std::fs::read(dir.path().join("binary.ts")).expect("read binary");
    assert!(
        !binary.windows(9).any(|w| w == b"components"),
        "binary must not be modified"
    );
}

#[test]
fn replace_respects_gitignore() {
    let dir = tempdir().expect("tempdir");
    support::init_git(dir.path());
    support::write_file(dir.path(), ".gitignore", "vendored/\n");
    support::write_file(
        dir.path(),
        "vendored/pkg/index.js",
        "const a = 'admin/resource-kit';\n",
    );
    support::write_file(
        dir.path(),
        "src/hit.ts",
        "import x from 'admin/resource-kit';\n",
    );

    let out = support::run(&[
        "--root",
        &dir.path().display().to_string(),
        "--yes",
        "replace",
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

    let ignored = std::fs::read_to_string(dir.path().join("vendored/pkg/index.js")).unwrap();
    assert!(
        ignored.contains("admin/resource-kit"),
        "gitignored file must be skipped"
    );
    let hit = support::read_file(dir.path(), "src/hit.ts");
    assert!(
        hit.contains("components/resource-kit"),
        "tracked file must be updated"
    );
}

#[test]
fn replace_is_idempotent() {
    let dir = tempdir().expect("tempdir");
    setup_repo(dir.path());

    for _ in 0..2 {
        let out = support::run(&[
            "--root",
            &dir.path().display().to_string(),
            "--yes",
            "replace",
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
    }

    let academic = support::read_file(dir.path(), "app/resources/js/admin/academic/columns.tsx");
    // exactly one occurrence of the new path
    assert_eq!(academic.matches("components/resource-kit").count(), 1);
    assert!(!academic.contains("admin/resource-kit"));
}
