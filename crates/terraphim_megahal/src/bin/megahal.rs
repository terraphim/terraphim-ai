//! Interactive `megahal` command-line chatbot (native target only).
//!
//! Mirrors the spirit of the upstream gem's `bin/megahal`: an interactive
//! loop with a `/`-command menu. Brains are stored in the crate's `MHRS1`
//! JSON format (upstream's zip+Marshal files are not portable).

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use rand_core::SeedableRng;
use terraphim_megahal::{MegaHal, MegahalError};
use terraphim_sooth::DefaultRng;

const HELP: &str = "\
commands:
  /help                    show this help
  /reset                   wipe the brain and reload the default personality
  /train <file>            train on a text file (one sentence per line)
  /save <file>             save the brain (MHRS1 JSON)
  /load <file>             load a brain saved by /save
  /personality <name>      switch personality (see /list)
  /list                    list available personalities
  /learning <on|off>       toggle reply-time learning
  /quit                    leave";

fn parse_seed() -> u64 {
    std::env::var("MEGAHAL_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(42)
}

fn main() -> ExitCode {
    let mut megahal = MegaHal::new();
    let mut rng = DefaultRng::seed_from_u64(parse_seed());
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout();
    let mut line = String::new();

    println!("MegaHAL (terraphim port). Type /help for commands.");
    loop {
        line.clear();
        print!("> ");
        let _ = stdout.flush();
        if stdin.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let command = parts.next().unwrap_or("");
        let argument = parts.next().unwrap_or("").trim();

        match command {
            "/quit" | "/exit" | "/bye" => break,
            "/help" => println!("{HELP}"),
            "/list" => {
                for name in MegaHal::personality_names() {
                    println!("  {name}");
                }
            }
            "/reset" => {
                megahal
                    .load_personality("default")
                    .expect("default personality always available");
                println!("Brain reset to the default personality.");
            }
            "/personality" => match megahal.load_personality(argument) {
                Ok(()) => println!("Personality switched to {argument}."),
                Err(error @ MegahalError::NoSuchPersonality(_)) => {
                    println!("{error}. Try /list.");
                }
                Err(error) => println!("{error}"),
            },
            "/learning" => match argument {
                "on" | "true" | "yes" => {
                    megahal.set_learning(true);
                    println!("Learning enabled.");
                }
                "off" | "false" | "no" => {
                    megahal.set_learning(false);
                    println!("Learning disabled.");
                }
                _ => println!("Usage: /learning <on|off>"),
            },
            "/train" => {
                let path = PathBuf::from(argument);
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        megahal.train(&text);
                        println!("Trained on {}.", path.display());
                    }
                    Err(error) => println!("Cannot read {}: {error}", path.display()),
                }
            }
            "/save" => {
                let path = PathBuf::from(argument);
                match std::fs::write(&path, megahal.save()) {
                    Ok(()) => println!("Brain saved to {}.", path.display()),
                    Err(error) => println!("Cannot write {}: {error}", path.display()),
                }
            }
            "/load" => {
                let path = PathBuf::from(argument);
                match std::fs::read_to_string(&path) {
                    Ok(json) => match megahal.load(&json) {
                        Ok(()) => println!("Brain loaded from {}.", path.display()),
                        Err(error) => println!("Cannot load: {error}"),
                    },
                    Err(error) => println!("Cannot read {}: {error}", path.display()),
                }
            }
            _ => {
                let reply = megahal.reply(Some(trimmed), &mut rng);
                println!("{reply}");
            }
        }
    }
    println!("Goodbye.");
    ExitCode::SUCCESS
}
