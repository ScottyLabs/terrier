use std::{fs, path::PathBuf, process::Command};

#[test]
fn searches_sections_with_status_defaults_and_plain_piped_output() {
    let root = std::env::temp_dir().join(format!("terrier-fetch-{}", std::process::id()));
    let directory = root.join("rfcs");
    fs::create_dir_all(&directory).unwrap();
    let source = "# RFC 0001: Storage\n- **Status:** Accepted\n\n## Design\nUse Redis for sessions.\n\n## Alternatives\nPostgres adds load.\n";
    fs::write(directory.join("0001-storage.md"), source).unwrap();
    fs::write(
        directory.join("0002-draft.md"),
        source.replace("Accepted", "Draft"),
    )
    .unwrap();
    fs::write(directory.join("README.md"), source).unwrap();

    let run = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_fetch"))
            .args(args)
            .env("DEVENV_ROOT", &root)
            .env("CLICOLOR", "0")
            .env_remove("CLICOLOR_FORCE")
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let output = run(&["STORAGE", "redis"]);
    assert!(output.contains("Use Redis for sessions."));
    assert!(output.contains("0001-storage.md:4"));
    assert!(output.contains("#design"));
    assert!(!output.contains("0002-draft"));
    assert!(!output.contains("README"));
    assert!(!output.contains('\x1b'));
    assert_eq!(run(&["redis", "postgres"]), "No matching RFCs.\n");
    assert!(run(&[]).contains("Accepted"));
    assert!(run(&["--status", "DRAFT"]).contains("0002-draft.md"));
    assert!(run(&["--status=all"]).contains("0002-draft.md"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn provides_help_and_rejects_invalid_options() {
    for args in [vec!["--unknown"], vec!["--status"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_fetch"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_fetch"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("accepted")
    );
}

#[test]
fn searches_repository_code_blocks_in_their_markdown_section() {
    let output = Command::new(env!("CARGO_BIN_EXE_fetch"))
        .args(["NixOS", "configuration"])
        .env(
            "DEVENV_ROOT",
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        )
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("#deployment")
    );
}
