mod parser;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {}", message);
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("get") => {
            let source = args
                .get(1)
                .ok_or_else(|| "usage: connstr get <FILE|-> <KEY>".to_string())?;
            let key = args
                .get(2)
                .ok_or_else(|| "usage: connstr get <FILE|-> <KEY>".to_string())?;
            cmd_get(source, key)
        }
        Some("keys") => {
            let source = args
                .get(1)
                .ok_or_else(|| "usage: connstr keys <FILE|->".to_string())?;
            cmd_keys(source)
        }
        Some("validate") => {
            let source = args
                .get(1)
                .ok_or_else(|| "usage: connstr validate <FILE|->".to_string())?;
            cmd_validate(source)
        }
        Some("--help") | Some("-h") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => Err(format!("unknown command '{}', try --help", other)),
    }
}

fn print_usage() {
    println!(
        "connstr - query one field out of a connection string\n\
         \n\
         usage:\n\
         \x20\x20connstr get <FILE|-> <KEY>   print the value of KEY (case-insensitive)\n\
         \x20\x20connstr keys <FILE|->        list every key found, in order\n\
         \x20\x20connstr validate <FILE|->    report every parse error found, not just the first\n\
         \n\
         FILE may be '-' to read the connection string from stdin."
    );
}

fn read_source(source: &str) -> Result<String, String> {
    if source == "-" {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {}", e))?;
        Ok(buf)
    } else {
        fs::read_to_string(source).map_err(|e| format!("failed to read '{}': {}", source, e))
    }
}

fn cmd_get(source: &str, key: &str) -> Result<(), String> {
    let input = read_source(source)?;
    let pairs = parser::parse(&input).map_err(|e| format!("{}: {}", source, e))?;

    match pairs.iter().find(|p| p.key.eq_ignore_ascii_case(key)) {
        Some(pair) => {
            println!("{}", pair.value);
            Ok(())
        }
        None => {
            let available: Vec<&str> = pairs.iter().map(|p| p.key.as_str()).collect();
            let available = if available.is_empty() {
                "(none)".to_string()
            } else {
                available.join(", ")
            };
            Err(format!(
                "key '{}' not found; available keys: {}",
                key, available
            ))
        }
    }
}

fn cmd_keys(source: &str) -> Result<(), String> {
    let input = read_source(source)?;
    let pairs = parser::parse(&input).map_err(|e| format!("{}: {}", source, e))?;
    for pair in pairs {
        println!("{}", pair.key);
    }
    Ok(())
}

fn cmd_validate(source: &str) -> Result<(), String> {
    let input = read_source(source)?;
    let errors = parser::validate(&input);
    if errors.is_empty() {
        println!("{}: ok", source);
        return Ok(());
    }

    for error in &errors {
        eprintln!("{}: {}", source, error);
    }
    Err(format!(
        "{} error{} found",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" }
    ))
}
