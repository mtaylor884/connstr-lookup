mod aliases;
mod json;
mod parser;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Text,
    Json,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            // Empty message means the command already reported the error
            // itself (e.g. as a JSON object), so don't double it up.
            if !message.is_empty() {
                eprintln!("error: {}", message);
            }
            ExitCode::FAILURE
        }
    }
}

// Pulls "--format text|json" (or "--format=text|json") out of args,
// wherever it appears, and returns it along with the remaining positional
// arguments.
fn extract_format(args: &[String]) -> Result<(Format, Vec<String>), String> {
    let mut format = Format::Text;
    let mut rest = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let value = if let Some(v) = arg.strip_prefix("--format=") {
            Some(v.to_string())
        } else if arg == "--format" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| "--format needs a value: text or json".to_string())?;
            Some(v.clone())
        } else {
            None
        };

        match value {
            Some(v) => {
                format = match v.as_str() {
                    "text" => Format::Text,
                    "json" => Format::Json,
                    other => {
                        return Err(format!("unknown --format '{}', expected text or json", other))
                    }
                };
            }
            None => rest.push(arg.clone()),
        }
        i += 1;
    }
    Ok((format, rest))
}

fn run(args: &[String]) -> Result<(), String> {
    let (format, args) = extract_format(args)?;
    match args.first().map(String::as_str) {
        Some("get") => {
            let source = args
                .get(1)
                .ok_or_else(|| "usage: connstr get <FILE|-> <KEY>".to_string())?;
            let key = args
                .get(2)
                .ok_or_else(|| "usage: connstr get <FILE|-> <KEY>".to_string())?;
            cmd_get(source, key, format)
        }
        Some("keys") => {
            let source = args
                .get(1)
                .ok_or_else(|| "usage: connstr keys <FILE|->".to_string())?;
            cmd_keys(source, format)
        }
        Some("validate") => {
            let source = args
                .get(1)
                .ok_or_else(|| "usage: connstr validate <FILE|->".to_string())?;
            cmd_validate(source, format)
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
         \x20\x20connstr get <FILE|-> <KEY>   print the value of KEY (case-insensitive, aliases like Server/Data Source match)\n\
         \x20\x20connstr keys <FILE|->        list every key found, in order\n\
         \x20\x20connstr validate <FILE|->    report every parse error found, not just the first\n\
         \n\
         FILE may be '-' to read the connection string from stdin.\n\
         Add --format json to any command to get machine-readable output instead of plain text."
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

fn report_parse_error(source: &str, error: &parser::ParseError, format: Format) -> String {
    match format {
        Format::Text => {
            eprintln!("{}: {}", source, error);
        }
        Format::Json => {
            let pos = error.position();
            eprintln!(
                "{{\"error\":{},\"line\":{},\"column\":{}}}",
                json::quoted(&error.message()),
                pos.line,
                pos.column
            );
        }
    }
    String::new()
}

fn cmd_get(source: &str, key: &str, format: Format) -> Result<(), String> {
    let input = read_source(source)?;
    let pairs = match parser::parse(&input) {
        Ok(pairs) => pairs,
        Err(e) => return Err(report_parse_error(source, &e, format)),
    };

    let target = aliases::canonical(key);
    match pairs.iter().find(|p| aliases::canonical(&p.key) == target) {
        Some(pair) => {
            match format {
                Format::Text => println!("{}", pair.value),
                Format::Json => println!(
                    "{{\"key\":{},\"value\":{}}}",
                    json::quoted(&pair.key),
                    json::quoted(&pair.value)
                ),
            }
            Ok(())
        }
        None => {
            let available: Vec<&str> = pairs.iter().map(|p| p.key.as_str()).collect();
            match format {
                Format::Text => {
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
                Format::Json => {
                    let available_json = available
                        .iter()
                        .map(|k| json::quoted(k))
                        .collect::<Vec<_>>()
                        .join(",");
                    eprintln!(
                        "{{\"error\":\"key '{}' not found\",\"available\":[{}]}}",
                        json::escape(key),
                        available_json
                    );
                    Err(String::new())
                }
            }
        }
    }
}

fn cmd_keys(source: &str, format: Format) -> Result<(), String> {
    let input = read_source(source)?;
    let pairs = match parser::parse(&input) {
        Ok(pairs) => pairs,
        Err(e) => return Err(report_parse_error(source, &e, format)),
    };
    match format {
        Format::Text => {
            for pair in pairs {
                println!("{}", pair.key);
            }
        }
        Format::Json => {
            let keys = pairs
                .iter()
                .map(|p| json::quoted(&p.key))
                .collect::<Vec<_>>()
                .join(",");
            println!("{{\"keys\":[{}]}}", keys);
        }
    }
    Ok(())
}

fn cmd_validate(source: &str, format: Format) -> Result<(), String> {
    let input = read_source(source)?;
    let errors = parser::validate(&input);
    if errors.is_empty() {
        match format {
            Format::Text => println!("{}: ok", source),
            Format::Json => println!("{{\"status\":\"ok\"}}"),
        }
        return Ok(());
    }

    match format {
        Format::Text => {
            for error in &errors {
                eprintln!("{}: {}", source, error);
            }
            Err(format!(
                "{} error{} found",
                errors.len(),
                if errors.len() == 1 { "" } else { "s" }
            ))
        }
        Format::Json => {
            let entries = errors
                .iter()
                .map(|e| {
                    let pos = e.position();
                    format!(
                        "{{\"message\":{},\"line\":{},\"column\":{}}}",
                        json::quoted(&e.message()),
                        pos.line,
                        pos.column
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            eprintln!("{{\"errors\":[{}]}}", entries);
            Err(String::new())
        }
    }
}
