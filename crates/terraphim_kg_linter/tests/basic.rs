use std::fs;
use std::io::Write;

#[tokio::test]
async fn parses_and_lints_minimal_schema() {
    let tmp = tempfile::tempdir().unwrap();
    let md_path = tmp.path().join("schema.md");
    let mut f = fs::File::create(&md_path).unwrap();
    writeln!(
        f,
        r#"# Example Schema

```kg-types
Document:
  id: string
  title: string
  tags: string[]
```

```kg-commands
name: search
description: Search docs
args:
  - name: query
    type: string
  - name: limit
    type: integer
    required: false
permissions:
  - can: read
    on: documents
```

```kg-permissions
roles:
  - name: agent
    allow:
      - action: execute
        command: search
      - action: read
        resource: documents
```
"#
    )
    .unwrap();

    let report = terraphim_kg_linter::lint_path(tmp.path()).await.unwrap();
    assert!(report.scanned_files >= 1);
    // We expect no issues for this simple, valid schema
    assert!(
        report.issues.is_empty(),
        "Unexpected issues: {:#?}",
        report.issues
    );
}
