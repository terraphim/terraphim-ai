//! Runs the built `terraphim-lsp` binary for `--version`/`--help` and checks
//! the `serverInfo` returned by `initialize`. No mocks: the real binary and
//! the real service are used.

use std::process::{Command, Stdio};

use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_types::Thesaurus;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_terraphim-lsp"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run terraphim-lsp")
}

#[test]
fn version_flags_print_name_and_version() {
    let expected = format!("terraphim-lsp {}\n", env!("CARGO_PKG_VERSION"));
    for flag in ["--version", "-V"] {
        let out = run(&[flag]);
        assert!(out.status.success(), "{flag} should exit 0");
        assert_eq!(String::from_utf8_lossy(&out.stdout), expected, "{flag}");
    }
}

#[test]
fn help_flags_describe_usage() {
    for flag in ["--help", "-h"] {
        let out = run(&[flag]);
        assert!(out.status.success(), "{flag} should exit 0");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("stdio"), "{flag}: {stdout}");
        assert!(stdout.contains("--thesaurus"), "{flag}: {stdout}");
        assert!(stdout.contains("TERRAPHIM_THESAURUS"), "{flag}: {stdout}");
    }
}

#[test]
fn missing_thesaurus_value_exits_with_usage_error() {
    let out = run(&["--thesaurus"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("Usage:"));
}

#[tokio::test]
async fn initialize_reports_server_info() {
    let (service, _socket) = LspService::new(|client| {
        TerraphimLspServer::new(client, Thesaurus::new("empty".to_string()))
    });
    let result = service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    let info = result.server_info.expect("serverInfo");
    assert_eq!(info.name, "terraphim-lsp");
    assert_eq!(info.version.as_deref(), Some(env!("CARGO_PKG_VERSION")));
}
