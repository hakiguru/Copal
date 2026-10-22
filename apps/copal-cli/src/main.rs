//! `copal-cli` — Copal from the terminal.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use copal_core::{Document, Session, Settings, registry, script::quote};
use copal_shell::{Reply, Shell};
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
    Info {
        file: PathBuf,
        /// Also print every project setting (print, supports, hollow, layout).
        #[arg(long)]
        settings: bool,
    },
    /// Add models (STL, OBJ, 3MF) to a project and save it.
    Import {
        file: PathBuf,
        #[arg(required = true)]
        models: Vec<PathBuf>,
        /// Arrange all objects on the plate afterwards.
        #[arg(long)]
        arrange: bool,
    },
    /// Slice a project and write the printer file (e.g. .goo). Also adds it to the print history.
    Export {
        file: PathBuf,
        /// Output file. Default: next to the project, named after it.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Don't add this export to the print history.
        #[arg(long)]
        no_history: bool,
    },
    /// Interactive shell: type commands, `help` lists them. Opens a project if one is given.
    Shell { file: Option<PathBuf> },
    /// Print history: exported files, their settings, ratings and project snapshots.
    ///
    /// copal-cli history | history show <id> | history rate <id> 5 good "note"
    /// | history open <id> <new.copal> | history remove <id> | history folder
    History { args: Vec<String> },
    /// Personal presets: list, save from a project, load into a project, delete.
    ///
    /// copal-cli presets | presets save <project> <kind> <name> | presets load <project> <kind> <name>
    /// | presets delete <kind> <name>     kinds: resin, exposure, motion, supports, hollow, layout
    Presets { args: Vec<String> },
    /// List known printers.
    Printers,
    /// List resins. Without --all, only resins with settings for --printer.
    Resins {
        #[arg(long)]
        printer: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Show the header of a printer file and check every layer; optionally save one layer as PNG.
    Inspect {
        file: PathBuf,
        /// Layer to save as an image.
        #[arg(long)]
        layer: Option<u32>,
        /// Where to save the layer image (PNG).
        #[arg(long)]
        png: Option<PathBuf>,
    },
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
    // Built-in printers and resins plus the user's own presets.
    copal_core::set_library(copal_core::load_user_library());
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
            let mut session = Session::new(Document::new(name));
            if let Some(p) = printer {
                session
                    .run_line(&format!("project.set_printer printer={}", quote(&p)))
                    .map_err(|e| e.to_string())?;
            }
            if let Some(r) = resin {
                session
                    .run_line(&format!("project.set_resin resin={}", quote(&r)))
                    .map_err(|e| e.to_string())?;
            }
            session.save_as(&file).map_err(|e| e.to_string())?;
            if cli.json {
                println!("{}", json!({ "created": file }));
            } else {
                println!("Created {}", file.display());
            }
            Ok(())
        }
        Cmd::Info { file, settings } => {
            let session = open(&file)?;
            print_info(&session, cli.json, settings);
            Ok(())
        }
        Cmd::Import {
            file,
            models,
            arrange,
        } => {
            let mut session = open(&file)?;
            for m in &models {
                session
                    .run_line(&format!(
                        "object.import path={}",
                        quote(&m.to_string_lossy())
                    ))
                    .map_err(|e| e.to_string())?;
            }
            if arrange {
                session
                    .run_line("object.arrange")
                    .map_err(|e| e.to_string())?;
            }
            finish(
                &mut session,
                false,
                models.len() + arrange as usize,
                cli.json,
            )
        }
        Cmd::Export {
            file,
            output,
            no_history,
        } => export(&file, output, no_history, cli.json),
        Cmd::Shell { file } => shell(file),
        Cmd::History { args } => {
            let mut sh = Shell::new(Session::default());
            if args.first().map(String::as_str) == Some("open") {
                // history open <id> <new.copal>: copy the snapshot to a new project file.
                let (Some(id), Some(dest)) = (args.get(1), args.get(2)) else {
                    return Err("usage: copal-cli history open <id> <new.copal>".into());
                };
                let h = sh
                    .history
                    .clone()
                    .ok_or("no print history on this system")?;
                h.get(id).map_err(|e| e.to_string())?;
                std::fs::copy(h.snapshot_path(id), dest).map_err(|e| e.to_string())?;
                println!("Saved the project as it was sliced to {dest}");
                return Ok(());
            }
            if cli.json && args.is_empty() {
                let list = sh.history.as_ref().map(|h| h.list()).unwrap_or_default();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?
                );
                return Ok(());
            }
            replies(sh.run(&shell_line("history", &args)))
        }
        Cmd::Presets { args } => presets(&args),
        Cmd::Printers => {
            let mut sh = Shell::new(Session::default());
            replies(sh.run("printers"))
        }
        Cmd::Resins { printer, all } => {
            let mut sh = Shell::new(Session::default());
            if let Some(p) = printer {
                sh.session.doc.printer = Some(p);
            }
            replies(sh.run(if all || sh.session.doc.printer.is_none() {
                "resins all"
            } else {
                "resins"
            }))
        }
        Cmd::Inspect { file, layer, png } => inspect(&file, layer, png, cli.json),
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

fn print_info(s: &Session, json_out: bool, with_settings: bool) {
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
            "settings": d.settings,
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
    if with_settings {
        let mut sh = Shell::new(Session::new(d.clone()));
        for r in sh.run("settings") {
            println!("{}", r.text());
        }
    }
}

/// Prints shell replies; fails if any of them is an error.
fn replies(rs: Vec<Reply>) -> Result<(), String> {
    let mut error = None;
    for r in rs {
        match r {
            Reply::Info(t) => println!("{t}"),
            Reply::Error(t) => error = Some(t),
        }
    }
    error.map_or(Ok(()), Err)
}

/// Builds a shell line from CLI arguments, quoting ones with spaces.
fn shell_line(cmd: &str, args: &[String]) -> String {
    let mut line = cmd.to_string();
    for a in args {
        line.push(' ');
        line.push_str(&quote(a));
    }
    line
}

fn export(
    file: &Path,
    output: Option<PathBuf>,
    no_history: bool,
    json_out: bool,
) -> Result<(), String> {
    let session = open(file)?;
    let mut sh = Shell::new(session);
    if no_history {
        sh.history = None;
    }
    let last = std::sync::atomic::AtomicU32::new(0);
    sh.progress = Box::new(move |f| {
        let pct = (f * 100.0) as u32;
        if pct / 10 != last.swap(pct, std::sync::atomic::Ordering::Relaxed) / 10 || pct == 100 {
            eprint!("\rSlicing… {pct:>3}%");
            if pct == 100 {
                eprintln!();
            }
        }
    });
    let out = match output {
        Some(p) => p,
        None => sh.default_export_path()?,
    };
    let rs = sh.run(&format!("export {}", quote(&out.to_string_lossy())));
    if json_out {
        let error = rs.iter().find_map(|r| match r {
            Reply::Error(e) => Some(e.clone()),
            _ => None,
        });
        if let Some(e) = error {
            return Err(e);
        }
        println!(
            "{}",
            json!({ "output": out, "messages": rs.iter().map(|r| r.text()).collect::<Vec<_>>() })
        );
        return Ok(());
    }
    replies(rs)
}

fn presets(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("save") | Some("load") => {
            let [action, project, kind, name @ ..] = args else {
                return Err(
                    "usage: copal-cli presets save|load <project.copal> <kind> <name>".into(),
                );
            };
            if name.is_empty() {
                return Err("give the preset a name".into());
            }
            let mut sh = Shell::new(open(Path::new(project))?);
            let mut a = vec![action.clone(), kind.clone()];
            a.extend(name.iter().cloned());
            replies(sh.run(&shell_line("presets", &a)))?;
            if action == "load" {
                sh.session.save().map_err(|e| e.to_string())?;
                println!("Saved {project}");
            }
            Ok(())
        }
        _ => {
            let mut sh = Shell::new(Session::default());
            replies(sh.run(&shell_line("presets", args)))
        }
    }
}

fn shell(file: Option<PathBuf>) -> Result<(), String> {
    use std::io::{BufRead, Write};
    let session = match &file {
        Some(f) if f.exists() => open(f)?,
        Some(f) => {
            let mut s = Session::new(Document::new(default_name(f)));
            s.save_as(f).map_err(|e| e.to_string())?;
            s
        }
        None => Session::default(),
    };
    let mut sh = Shell::new(session);
    sh.progress = Box::new(|f| eprint!("\rSlicing… {:>3}%", (f * 100.0) as u32));
    println!(
        "Copal {} shell. `help` lists commands, `exit` quits.",
        copal_core::VERSION
    );
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    loop {
        print!("copal:{}$ ", sh.place());
        std::io::stdout().flush().ok();
        let Some(Ok(line)) = lines.next() else { break };
        if matches!(line.trim(), "exit" | "quit") {
            break;
        }
        for r in sh.run(&line) {
            match r {
                Reply::Info(t) => println!("{t}"),
                Reply::Error(t) => println!("error: {t}"),
            }
        }
    }
    if sh.session.is_dirty() && sh.session.path().is_some() {
        eprintln!("Unsaved changes. Type `save` before `exit` next time; they were not saved.");
    }
    Ok(())
}

fn inspect(
    file: &Path,
    layer: Option<u32>,
    png: Option<PathBuf>,
    json_out: bool,
) -> Result<(), String> {
    use copal_formats::goo::GooFile;
    let reader = std::io::BufReader::new(
        std::fs::File::open(file).map_err(|e| format!("{}: {e}", file.display()))?,
    );
    let mut f = GooFile::open(reader).map_err(|e| format!("{}: {e}", file.display()))?;
    let h = f.header.clone();
    let (w, ht) = (h.resolution[0] as usize, h.resolution[1] as usize);
    let mut lit = 0f64;
    for i in 0..f.layer_count() {
        lit += f.layer_lit(i).map_err(|e| e.to_string())?;
    }
    if let (Some(l), Some(out)) = (layer, &png) {
        let (_, img) = f.layer(l).map_err(|e| e.to_string())?;
        save_png(out, &img, w as u32, ht as u32)?;
    }
    let px = h.size_mm[0] as f64 / w as f64 * h.size_mm[1] as f64 / ht as f64;
    let volume_ml = lit * px * h.layer_height_mm as f64 / 1000.0;
    if json_out {
        println!(
            "{}",
            json!({
                "software": format!("{} {}", h.software_info, h.software_version),
                "printer": h.printer_name, "profile": h.profile_name,
                "layers": h.layer_count, "resolution": h.resolution,
                "layer_height_mm": h.layer_height_mm, "exposure_s": h.exposure_s,
                "bottom_layers": h.bottom_layers, "bottom_exposure_s": h.bottom_exposure_s,
                "volume_ml": volume_ml, "print_time_s": h.print_time_s,
            })
        );
    } else {
        println!("File      {}", file.display());
        println!(
            "Made by   {} {} ({})",
            h.software_info, h.software_version, h.file_time
        );
        println!(
            "Printer   {} — {}×{} px, {:?} mm",
            h.printer_name, w, ht, h.size_mm
        );
        println!("Profile   {}", h.profile_name);
        println!(
            "Layers    {} × {} mm, exposure {} s, bottom {} × {} s, transition {}",
            h.layer_count,
            h.layer_height_mm,
            h.exposure_s,
            h.bottom_layers,
            h.bottom_exposure_s,
            h.transition_layers
        );
        println!(
            "Lift      bottom {:?} / {:?}, normal {:?} / {:?} (mm, mm/min; lift / retract)",
            h.bottom_lift, h.bottom_retract, h.lift, h.retract
        );
        println!(
            "Resin     {volume_ml:.2} ml, print time {}",
            copal_core::time::format_duration(h.print_time_s as f64)
        );
        println!("All {} layers decode correctly.", h.layer_count);
        if let (Some(l), Some(p)) = (layer, &png) {
            println!("Layer {l} saved to {}", p.display());
        }
    }
    Ok(())
}

fn save_png(path: &Path, grey: &[u8], w: u32, h: u32) -> Result<(), String> {
    let rgb: Vec<u8> = grey.iter().flat_map(|g| [*g, *g, *g]).collect();
    std::fs::write(path, copal_formats::export::png(&rgb, w, h)).map_err(|e| e.to_string())
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
