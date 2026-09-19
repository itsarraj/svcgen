use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use svcgen::generate::{generate_unit_file, ServiceConfig};
use svcgen::parse::parse_unit_file;
use svcgen::validate::{validate, Severity};

#[derive(Parser)]
#[command(
    name = "svcgen",
    about = "Generates a systemd unit file with sane defaults, and validates an existing one"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a new .service unit file.
    Generate {
        #[arg(long)]
        exec: String,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        workdir: Option<String>,
        /// KEY=VALUE, repeatable.
        #[arg(long = "env")]
        env: Vec<String>,
        #[arg(long, default_value_t = 5)]
        restart_sec: u32,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Check an existing unit file for common mistakes.
    Validate { file: PathBuf },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate {
            exec,
            description,
            user,
            workdir,
            env,
            restart_sec,
            out,
        } => {
            let description = if description.is_empty() {
                exec.clone()
            } else {
                description
            };
            let env_pairs: Vec<(String, String)> = env
                .iter()
                .filter_map(|kv| {
                    kv.split_once('=')
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                })
                .collect();
            let cfg = ServiceConfig {
                description,
                exec_start: exec,
                working_directory: workdir,
                user,
                env: env_pairs,
                restart_sec,
            };
            let unit = generate_unit_file(&cfg);
            match out {
                Some(path) => {
                    fs::write(&path, &unit)?;
                    println!("wrote {}", path.display());
                }
                None => print!("{unit}"),
            }
        }
        Command::Validate { file } => {
            let content = fs::read_to_string(&file)
                .map_err(|e| anyhow::anyhow!("reading {}: {e}", file.display()))?;
            let unit = parse_unit_file(&content);
            let issues = validate(&unit);
            if issues.is_empty() {
                println!("no issues found");
            } else {
                let mut has_error = false;
                for issue in &issues {
                    let tag = match issue.severity {
                        Severity::Error => {
                            has_error = true;
                            "ERROR"
                        }
                        Severity::Warning => "WARN ",
                    };
                    println!("{tag}  {}", issue.message);
                }
                std::process::exit(if has_error { 2 } else { 1 });
            }
        }
    }
    Ok(())
}
