//! `copal-cli` — Copal from the terminal.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use copal_core::{Document, Session, Settings, registry};
use serde_json::json;

/// Copal — a free slicer for resin 3D printers. Everything the apps can do, from the terminal.
#[derive(Parser)]
#[command(name = "copal-cli", version, about, long_about = None)]
struct Cli {
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new empty project.
    New {
        /// Where to save the project (.copal).
        file: PathBuf,
        /// Project name (defaults to the file name).
        #[arg(long)]
        name: Option<String>,
        /// Printer preset id, e.g. elegoo/saturn-4-ultra-16k.
        #[arg(long)]
        printer: Option<String>,
        /// Resin preset id, e.g. anycubic/abs-like-pro-2-grey.
        #[arg(long)]
        resin: Option<String>,
        /// Overwrite the file if it exists.
        #[arg(long)]
        force: bool,
    },
    /// Show what is inside a project.
    Info { file: PathBuf },
    /// Run commands on a project and save it.
    ///
    /// Example: copal-cli run part.copal object.move id=1 x=10
    Run {
        file: PathBuf,
        /// One command in the text language: a name and key=value arguments.
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
        command: Vec<String>,
        /// Do everything but don't save.
        #[arg(long)]
        dry_run: bool,
    },
    /// Run a .copalscript file (one command per line) on a project and save it.
    Script {
        file: PathBuf,
        script: PathBuf,
        /// Do everything but don't save.
        #[arg(long)]
        dry_run: bool,
    },
    /// List all commands of the text language.
    Commands,
    /// Show or change app settings, e.g. `copal-cli settings theme=art mode=light`.
    Settings {
        /// Changes as key=value. Without any, prints the current settings.
        changes: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Cmd::New {
            file,
            name,
            printer,
            resin,
            force,
        } => {
            if file.exists() && !force {
                return Err(format!(
                    "{} already exists (use --force to overwrite)",
                    file.display()
                ));
            }
            let name = name.unwrap_or_else(|| default_name(&file));
            let mut doc = Document::new(name);
            doc.printer = printer;
            doc.resin = resin;
            let mut session = Session::new(doc);
            session.save_as(&file).map_err(|e| e.to_string())?;
            if cli.json {
                println!("{}", json!({ "created": file }));
            } else {
                println!("Created {}", file.display());
            }
            Ok(())
        }
        Cmd::Info { file } => {
            let session = open(&file)?;
            print_info(&session, cli.json);
            Ok(())
        }
        Cmd::Run {
            file,
            command,
            dry_run,
        } => {
            let mut session = open(&file)?;
            let line = command
                .iter()
                .map(|a| quote_cli_arg(a))
                .collect::<Vec<_>>()
                .join(" ");
            session.run_line(&line).map_err(|e| e.to_string())?;
            finish(&mut session, dry_run, 1, cli.json)
        }
        Cmd::Script {
            file,
            script,
            dry_run,
        } => {
            let mut session = open(&file)?;
            let text = std::fs::read_to_string(&script)
                .map_err(|e| format!("cannot read {}: {e}", script.display()))?;
            let count = session
                .run_script(&text)
                .map_err(|e| format!("{}: {e}", script.display()))?;
            finish(&mut session, dry_run, count, cli.json)
        }
        Cmd::Settings { changes } => settings(&changes, cli.json),
        Cmd::Commands => {
            if cli.json {
                let list: Vec<_> = registry()
                    .iter()
                    .map(|c| json!({ "name": c.name, "summary": c.summary, "usage": c.usage }))
                    .collect();
                println!("{}", serde_json::Value::Array(list));
            } else {
                for c in registry() {
                    println!("{:<22} {}\n{:<22} {}\n", c.name, c.summary, "", c.usage);
                }
            }
            Ok(())
        }
    }
}

fn settings(changes: &[String], json_out: bool) -> Result<(), String> {
    let path = Settings::path().map_err(|e| e.to_string())?;
    let mut s = Settings::load_from(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    for change in changes {
        let (key, value) = change
            .split_once('=')
            .ok_or_else(|| format!("expected key=value, got `{change}`"))?;
        s.set(key, value).map_err(|e| e.to_string())?;
    }
    if !changes.is_empty() {
        s.save_to(&path).map_err(|e| e.to_string())?;
    }
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?
        );
    } else {
        for (key, values) in Settings::KEYS {
            println!(
                "{key:<6} = {:<8} ({values})",
                s.get(key).unwrap_or_default()
            );
        }
        println!("file   = {}", path.display());
    }
    Ok(())
}

fn open(file: &Path) -> Result<Session, String> {
    Session::open(file).map_err(|e| format!("{}: {e}", file.display()))
}

fn finish(session: &mut Session, dry_run: bool, count: usize, json: bool) -> Result<(), String> {
    if !dry_run {
        session.save().map_err(|e| e.to_string())?;
    }
    if json {
        println!("{}", json!({ "applied": count, "saved": !dry_run }));
    } else {
        let what = if count == 1 { "command" } else { "commands" };
        let saved = if dry_run {
            " (dry run, not saved)"
        } else {
            ", saved"
        };
        println!("Applied {count} {what}{saved}");
    }
    Ok(())
}

fn print_info(s: &Session, json_out: bool) {
    let d = &s.doc;
    if json_out {
        let objects: Vec<_> = d
            .objects()
            .iter()
            .map(|o| {
                let t = &o.transform;
                json!({
                    "id": o.id.0,
                    "name": o.name,
                    "triangles": o.mesh.triangle_count(),
                    "position": t.translation.to_array(),
                    "rotation_deg": t.rotation_deg.to_array(),
                    "scale": t.scale.to_array(),
                })
            })
            .collect();
        let info = json!({
            "name": d.name,
            "printer": d.printer,
            "resin": d.resin,
            "objects": objects,
        });
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
        return;
    }
    println!("Project  {}", d.name);
    println!("Printer  {}", d.printer.as_deref().unwrap_or("(not set)"));
    println!("Resin    {}", d.resin.as_deref().unwrap_or("(not set)"));
    println!("Objects  {}", d.objects().len());
    for o in d.objects() {
        let size = o
            .world_bounds()
            .map(|b| {
                let s = b.size();
                format!("{:.2} × {:.2} × {:.2} mm", s.x, s.y, s.z)
            })
            .unwrap_or_else(|| "empty".into());
        println!(
            "  [{}] {} — {}, {} triangles",
            o.id,
            o.name,
            size,
            o.mesh.triangle_count()
        );
    }
}

/// Re-quotes a shell argument that contains spaces, e.g. `name=My part` → `name="My part"`.
fn quote_cli_arg(arg: &str) -> String {
    match arg.split_once('=') {
        Some((k, v)) if v.chars().any(char::is_whitespace) || v.is_empty() => {
            format!("{k}=\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
        }
        _ => arg.to_string(),
    }
}

fn default_name(file: &Path) -> String {
    file.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}
