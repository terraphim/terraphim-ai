/// Command-line interface for Terraphim Build Argument Management
///
/// This CLI tool provides centralized management of build arguments for the
/// Terraphim AI project, supporting configuration of features, targets, and environments.
use clap::{Arg, Command};
use std::process;
use std::str::FromStr;
use terraphim_build_args::*;

fn main() {
    env_logger::init();

    let matches = Command::new("terraphim-build-args")
        .version("0.1.0")
        .author("Terraphim Contributors")
        .about("Build argument management for Terraphim AI")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .required(false),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FORMAT")
                .help("Output format: cargo, docker, earthly, json")
                .default_value("cargo"),
        )
        .arg(
            Arg::new("target")
                .short('t')
                .long("target")
                .value_name("TARGET")
                .help("Build target (e.g., native-release, cross-x86_64-unknown-linux-musl)")
                .required(false),
        )
        .arg(
            Arg::new("features")
                .short('f')
                .long("features")
                .value_name("FEATURES")
                .help("Comma-separated list of features to enable")
                .required(false),
        )
        .arg(
            Arg::new("environment")
                .short('e')
                .long("environment")
                .value_name("ENV")
                .help("Environment name (development, production, etc.)")
                .required(false),
        )
        .arg(
            Arg::new("validate")
                .long("validate")
                .help("Validate configuration only")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Load configuration from file if provided
    let mut config = if let Some(config_file) = matches.get_one::<String>("config") {
        match BuildConfig::from_file(config_file) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error loading config file: {}", e);
                process::exit(1);
            }
        }
    } else {
        BuildConfig::builder().build().unwrap_or_else(|e| {
            eprintln!("Error creating default config: {}", e);
            process::exit(1);
        })
    };

    // Override with command line arguments
    if let Some(target) = matches.get_one::<String>("target") {
        match BuildTarget::from_str(target) {
            Ok(t) => config.target = t,
            Err(e) => {
                eprintln!("Error parsing target: {}", e);
                process::exit(1);
            }
        }
    }

    if let Some(features) = matches.get_one::<String>("features") {
        match FeatureSet::from_string(features) {
            Ok(f) => config.features = f,
            Err(e) => {
                eprintln!("Error parsing features: {}", e);
                process::exit(1);
            }
        }
    }

    if let Some(environment) = matches.get_one::<String>("environment") {
        match Environment::from_str(environment) {
            Ok(e) => config.environment = e,
            Err(e) => {
                eprintln!("Error parsing environment: {}", e);
                process::exit(1);
            }
        }
    }

    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("Configuration validation failed: {}", e);
        process::exit(1);
    }

    // If validation only, exit successfully
    if matches.get_flag("validate") {
        println!("Configuration validation successful");
        return;
    }

    // Generate output based on format
    let output_format = matches.get_one::<String>("output").unwrap();
    match output_format.as_str() {
        "cargo" => {
            let args = config.cargo_args();
            println!("{}", args.join(" "));
        }
        "docker" => {
            let args = config.docker_args();
            println!("docker {}", args.join(" "));
        }
        "earthly" => {
            let args = config.earthly_args();
            println!("earthly {}", args.join(" "));
        }
        "json" => match serde_json::to_string_pretty(&config) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("Error serializing config to JSON: {}", e);
                process::exit(1);
            }
        },
        _ => {
            eprintln!("Unknown output format: {}", output_format);
            process::exit(1);
        }
    }
}
