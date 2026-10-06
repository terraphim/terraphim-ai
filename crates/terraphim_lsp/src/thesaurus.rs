//! Where the server's thesaurus comes from, and loading it from a file.
//!
//! The thesaurus is a JSON file in the format terraphim's thesaurus builders
//! write (`{"name": ..., "data": {"term": {"id": 1, "nterm": ...}}}`). Its
//! path is resolved in this order, the first one set winning:
//!
//! 1. the `thesaurus` setting (`initializationOptions` or
//!    `workspace/didChangeConfiguration`, bare or under `terraphim`);
//! 2. the `--thesaurus <path>` command-line flag;
//! 3. the `TERRAPHIM_THESAURUS` environment variable.
//!
//! Empty values are ignored. A leading `~/` expands to the home directory;
//! other relative paths are relative to the server's working directory
//! (editors usually start it at the workspace root). With none set, the
//! server runs with the thesaurus it was constructed with (empty for the
//! `terraphim-lsp` binary).
//!
//! Resolution is pure (it takes the environment value as input); only
//! [`load_thesaurus`] touches the file system. The analysis core stays free
//! of I/O.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};

use terraphim_lsp_core::{CoreError, KgEngine};
use terraphim_types::Thesaurus;

/// The environment variable naming a thesaurus JSON file.
pub const THESAURUS_ENV: &str = "TERRAPHIM_THESAURUS";

/// The command-line flag naming a thesaurus JSON file.
pub const THESAURUS_FLAG: &str = "--thesaurus";

/// The thesaurus path chosen at launch, before any client settings: from
/// the `--thesaurus` flag, else from [`THESAURUS_ENV`].
///
/// `home` expands a leading `~/`. Returns `None` when neither is set (or
/// both are empty).
pub fn launch_thesaurus_path(
    flag: Option<&OsStr>,
    env: Option<&OsStr>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    [flag, env]
        .into_iter()
        .flatten()
        .find(|value| !value.is_empty())
        .map(|value| expand_home(Path::new(value), home))
}

/// The thesaurus path in effect: the `thesaurus` setting if set (and not
/// blank), else the launch path from [`launch_thesaurus_path`].
pub fn effective_thesaurus_path(
    setting: Option<&str>,
    launch: Option<&Path>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    match setting.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Some(expand_home(Path::new(value), home)),
        None => launch.map(Path::to_path_buf),
    }
}

/// `path` with a leading `~` component replaced by `home` (when known).
fn expand_home(path: &Path, home: Option<&Path>) -> PathBuf {
    match (path.strip_prefix("~"), home) {
        (Ok(rest), Some(home)) => home.join(rest),
        _ => path.to_path_buf(),
    }
}

/// What the server knows at launch, before any client settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchOptions {
    /// The launch thesaurus path ([`launch_thesaurus_path`]), used when the
    /// client sets no `thesaurus`.
    pub thesaurus: Option<PathBuf>,
    /// The home directory, for expanding `~/` in the `thesaurus` setting.
    pub home: Option<PathBuf>,
}

impl LaunchOptions {
    /// Launch options from parsed arguments plus the given environment
    /// values (`TERRAPHIM_THESAURUS` and `HOME`), passed in so resolution
    /// stays testable without touching the process environment.
    pub fn new(cli: &CliArgs, env_thesaurus: Option<&OsStr>, home: Option<&OsStr>) -> Self {
        let home = home.filter(|home| !home.is_empty()).map(PathBuf::from);
        Self {
            thesaurus: launch_thesaurus_path(
                cli.thesaurus.as_deref(),
                env_thesaurus,
                home.as_deref(),
            ),
            home,
        }
    }

    /// Launch options from parsed arguments and this process' environment.
    pub fn from_process(cli: &CliArgs) -> Self {
        let env_thesaurus = std::env::var_os(THESAURUS_ENV);
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
        Self::new(cli, env_thesaurus.as_deref(), home.as_deref())
    }
}

/// Parsed command-line arguments of the `terraphim-lsp` binary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliArgs {
    /// `--thesaurus <path>` or `--thesaurus=<path>`.
    pub thesaurus: Option<OsString>,
    /// `--help` or `-h`.
    pub help: bool,
    /// `--version` or `-V`.
    pub version: bool,
    /// Arguments that were not recognised (such as `--stdio`, which some
    /// clients pass; the server always speaks stdio). Ignored with a log
    /// line rather than refused.
    pub ignored: Vec<OsString>,
}

/// Usage text for `terraphim-lsp --help`.
pub const USAGE: &str = "\
terraphim-lsp: Terraphim knowledge-graph language server (LSP over stdio)

Usage: terraphim-lsp [--thesaurus <path>]

Options:
  --thesaurus <path>  thesaurus JSON file (overridden by the client's
                      `thesaurus` setting; falls back to $TERRAPHIM_THESAURUS)
  -h, --help          print this help
  -V, --version       print the version
";

/// Parse the binary's arguments (without the program name).
///
/// # Errors
///
/// When `--thesaurus` is the last argument and has no value.
pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<CliArgs, String> {
    let mut parsed = CliArgs::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some(flag) if flag == THESAURUS_FLAG => {
                let value = args
                    .next()
                    .ok_or_else(|| format!("{THESAURUS_FLAG} needs a path"))?;
                parsed.thesaurus = Some(value);
            }
            Some(flag) if flag.starts_with("--thesaurus=") => {
                parsed.thesaurus = Some(OsString::from(&flag["--thesaurus=".len()..]));
            }
            Some("--help" | "-h") => parsed.help = true,
            Some("--version" | "-V") => parsed.version = true,
            _ => parsed.ignored.push(arg),
        }
    }
    Ok(parsed)
}

/// Why a thesaurus file could not be loaded.
#[derive(Debug)]
pub enum ThesaurusLoadError {
    /// The file could not be read.
    Read(std::io::Error),
    /// The file is not a thesaurus JSON document.
    Parse(serde_json::Error),
    /// The thesaurus could not be compiled into a matcher.
    Engine(CoreError),
}

impl fmt::Display for ThesaurusLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(f, "cannot read the file: {error}"),
            Self::Parse(error) => write!(f, "not a thesaurus JSON file: {error}"),
            Self::Engine(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ThesaurusLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Parse(error) => Some(error),
            Self::Engine(error) => Some(error),
        }
    }
}

/// A thesaurus and the engine compiled from it, with the file it came from.
#[derive(Debug)]
pub struct LoadedThesaurus {
    /// The thesaurus, for completion.
    pub thesaurus: Thesaurus,
    /// The compiled engine, for matching, hover, actions and hints.
    pub engine: KgEngine,
}

/// Read, parse and compile the thesaurus at `path`. Blocking: call it on a
/// blocking thread from async code.
///
/// # Errors
///
/// A read, parse or matcher-compilation failure.
pub fn load_thesaurus(path: &Path) -> Result<LoadedThesaurus, ThesaurusLoadError> {
    let json = std::fs::read_to_string(path).map_err(ThesaurusLoadError::Read)?;
    let thesaurus: Thesaurus = serde_json::from_str(&json).map_err(ThesaurusLoadError::Parse)?;
    let engine = KgEngine::new(&thesaurus).map_err(ThesaurusLoadError::Engine)?;
    Ok(LoadedThesaurus { thesaurus, engine })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../terraphim_lsp_core/tests/fixtures/writing_thesaurus.json"
    );

    fn os(value: &str) -> Option<&OsStr> {
        Some(OsStr::new(value))
    }

    #[test]
    fn flag_wins_over_env_and_empty_values_are_ignored() {
        let home = Path::new("/home/me");
        assert_eq!(
            launch_thesaurus_path(os("/a.json"), os("/b.json"), Some(home)),
            Some(PathBuf::from("/a.json"))
        );
        assert_eq!(
            launch_thesaurus_path(None, os("/b.json"), Some(home)),
            Some(PathBuf::from("/b.json"))
        );
        assert_eq!(
            launch_thesaurus_path(os(""), os("/b.json"), Some(home)),
            Some(PathBuf::from("/b.json"))
        );
        assert_eq!(launch_thesaurus_path(os(""), os(""), Some(home)), None);
        assert_eq!(launch_thesaurus_path(None, None, Some(home)), None);
    }

    #[test]
    fn setting_wins_over_launch_path() {
        let launch = Path::new("/launch.json");
        assert_eq!(
            effective_thesaurus_path(Some("/setting.json"), Some(launch), None),
            Some(PathBuf::from("/setting.json"))
        );
        assert_eq!(
            effective_thesaurus_path(Some("  "), Some(launch), None),
            Some(PathBuf::from("/launch.json"))
        );
        assert_eq!(
            effective_thesaurus_path(None, Some(launch), None),
            Some(PathBuf::from("/launch.json"))
        );
        assert_eq!(effective_thesaurus_path(None, None, None), None);
    }

    #[test]
    fn tilde_expands_to_home_when_known() {
        let home = Path::new("/home/me");
        assert_eq!(
            effective_thesaurus_path(Some("~/kg/t.json"), None, Some(home)),
            Some(PathBuf::from("/home/me/kg/t.json"))
        );
        assert_eq!(
            launch_thesaurus_path(None, os("~/t.json"), Some(home)),
            Some(PathBuf::from("/home/me/t.json"))
        );
        assert_eq!(
            effective_thesaurus_path(Some("~/t.json"), None, None),
            Some(PathBuf::from("~/t.json"))
        );
        assert_eq!(
            effective_thesaurus_path(Some("rel/~/t.json"), None, Some(home)),
            Some(PathBuf::from("rel/~/t.json"))
        );
    }

    #[test]
    fn launch_options_take_flag_then_env() {
        let cli = CliArgs {
            thesaurus: Some(OsString::from("~/flag.json")),
            ..CliArgs::default()
        };
        let options = LaunchOptions::new(&cli, os("/env.json"), os("/home/me"));
        assert_eq!(options.thesaurus, Some(PathBuf::from("/home/me/flag.json")));
        assert_eq!(options.home, Some(PathBuf::from("/home/me")));
        let options = LaunchOptions::new(&CliArgs::default(), os("/env.json"), os(""));
        assert_eq!(options.thesaurus, Some(PathBuf::from("/env.json")));
        assert_eq!(options.home, None);
    }

    #[test]
    fn parses_flags() {
        let args = |list: &[&str]| parse_args(list.iter().map(OsString::from));
        assert_eq!(args(&[]).unwrap(), CliArgs::default());
        assert_eq!(
            args(&["--thesaurus", "/t.json"]).unwrap().thesaurus,
            Some(OsString::from("/t.json"))
        );
        assert_eq!(
            args(&["--thesaurus=/t.json"]).unwrap().thesaurus,
            Some(OsString::from("/t.json"))
        );
        let parsed = args(&["--stdio", "-h", "-V"]).unwrap();
        assert!(parsed.help && parsed.version);
        assert_eq!(parsed.ignored, [OsString::from("--stdio")]);
        assert!(args(&["--thesaurus"]).is_err());
    }

    #[test]
    fn loads_the_fixture_thesaurus() {
        let loaded = load_thesaurus(Path::new(FIXTURE)).expect("fixture loads");
        assert!(!loaded.thesaurus.is_empty());
        assert!(!loaded.engine.concept_index().is_empty());
    }

    #[test]
    fn load_errors_are_distinguished() {
        let missing = load_thesaurus(Path::new("/nonexistent/terraphim/t.json")).unwrap_err();
        assert!(matches!(missing, ThesaurusLoadError::Read(_)), "{missing}");
        let invalid = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/invalid_thesaurus.json"
        );
        let error = load_thesaurus(Path::new(invalid)).unwrap_err();
        assert!(matches!(error, ThesaurusLoadError::Parse(_)), "{error}");
        assert!(error.to_string().starts_with("not a thesaurus JSON file"));
    }
}
