//! Embedded training corpora (personalities) from the upstream MegaHAL gem
//! (Unlicense), extracted by `scripts/extract_data.py` into
//! `src/personalities_data/*.txt` -- one sentence per line.
//!
//! The `:default` corpus is always embedded so that [`crate::MegaHal::new`]
//! matches upstream's `MegaHAL.new`. The remaining eleven personalities are
//! gated behind the `personalities` feature to keep the default binary small.

/// The `:default` personality corpus (always embedded).
pub const DEFAULT: &str = include_str!("personalities_data/default.txt");

/// Terraphim-authored corpus teaching the engine that Rust is a computer
/// language (not upstream; hand-written, one sentence per line). Always
/// embedded so the demo can showcase a domain persona.
pub const RUST: &str = include_str!("personalities_data/rust.txt");

/// All non-default personality corpora, gated behind the `personalities`
/// feature.
#[cfg(feature = "personalities")]
pub mod extra {
    pub const ALIENS: &str = include_str!("personalities_data/aliens.txt");
    pub const BILL: &str = include_str!("personalities_data/bill.txt");
    pub const CAITSITH: &str = include_str!("personalities_data/caitsith.txt");
    pub const FERRIS: &str = include_str!("personalities_data/ferris.txt");
    pub const MANSON: &str = include_str!("personalities_data/manson.txt");
    pub const PEPYS: &str = include_str!("personalities_data/pepys.txt");
    pub const PULP: &str = include_str!("personalities_data/pulp.txt");
    pub const SCREAM: &str = include_str!("personalities_data/scream.txt");
    pub const SHERLOCK: &str = include_str!("personalities_data/sherlock.txt");
    pub const STARTREK: &str = include_str!("personalities_data/startrek.txt");
    pub const STARWARS: &str = include_str!("personalities_data/starwars.txt");
}

/// Upstream personality names paired with their corpora. Without the
/// `personalities` feature only `default` is available.
pub fn available() -> Vec<(&'static str, &'static str)> {
    #[allow(unused_mut)] // mut is used only with the `personalities` feature
    let mut list = vec![("default", DEFAULT), ("rust", RUST)];
    #[cfg(feature = "personalities")]
    {
        list.extend([
            ("aliens", extra::ALIENS),
            ("bill", extra::BILL),
            ("caitsith", extra::CAITSITH),
            ("ferris", extra::FERRIS),
            ("manson", extra::MANSON),
            ("pepys", extra::PEPYS),
            ("pulp", extra::PULP),
            ("scream", extra::SCREAM),
            ("sherlock", extra::SHERLOCK),
            ("startrek", extra::STARTREK),
            ("starwars", extra::STARWARS),
        ]);
    }
    list
}

/// Look up a personality corpus by name.
pub fn corpus(name: &str) -> Option<&'static str> {
    available()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| c)
}
