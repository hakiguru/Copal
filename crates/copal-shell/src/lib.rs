//! The Copal shell: one set of commands for the console in both apps, `copal-cli shell` and
//! scripts.
//!
//! A line is either a shell command (`help`, `undo`, `save`, `export`, `presets`, `history`, …)
//! or a document command from the core registry (`object.move id=1 x=5`, `print.set …`).

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use copal_core::presets::{
    ExposureSettings, HollowSettings, Kind, LayoutSettings, MotionSettings, Resin, ResinProfile,
    SupportSettings, slug,
};
use copal_core::prints::{Outcome, PrintHistory};
use copal_core::time::{DateTime, format_duration};
use copal_core::{Document, ObjectId, Section, Session, SetSettings, library, script::quote};
use serde::Serialize;

/// One line of output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Info(String),
    Error(String),
}

impl Reply {
    pub fn text(&self) -> &str {
        match self {
            Reply::Info(t) | Reply::Error(t) => t,
        }
    }
}

/// Shell commands, for help and Tab completion.
pub const COMMANDS: &[(&str, &str)] = &[
    ("commands", "list every document command"),
    (
        "export",
        "export [file]  slice and write the printer file (also saved to the history)",
    ),
    (
        "help",
        "help [command]  this list, or how to use one command",
    ),
    (
        "history",
        "history [show|rate|open|remove] …  exported files and how they printed",
    ),
    ("import", "import <file> …  add STL, OBJ or 3MF models"),
    (
        "hollow",
        "hollow [id|all]  hollow models with the wall thickness from the Hollow settings",
    ),
    ("info", "info  the project, its objects and estimates"),
    (
        "islands",
        "islands [id|all]  find islands that no support holds",
    ),
    ("new", "new [name]  start an empty project"),
    ("open", "open <file.copal>  open a project"),
    (
        "presets",
        "presets [save|load|delete <kind> <name> | import <file.toml> | folder | reload]  your saved settings and resins",
    ),
    ("printers", "printers  every printer Copal knows"),
    ("redo", "redo  repeat the last undone step"),
    (
        "resins",
        "resins [all]  resins with settings for this printer",
    ),
    ("save", "save [file.copal]  save the project"),
    (
        "settings",
        "settings [print|supports|hollow|layout]  show project settings",
    ),
    ("steps", "steps  what undo would revert, newest last"),
    (
        "supports",
        "supports [auto|islands|clear] [id|all]  add or remove supports; no action lists them",
    ),
    ("undo", "undo  revert the last step"),
];

/// Kinds of presets the shell can save and load.
pub const PRESET_KINDS: &[&str] = &[
    "resin", "exposure", "motion", "supports", "hollow", "layout",
];

pub struct Shell {
    pub session: Session,
    /// Where exports are recorded. `None` turns the history off.
    pub history: Option<PrintHistory>,
    /// Called with 0..1 while exporting.
    pub progress: Box<dyn Fn(f32) + Sync + Send>,
}

impl Shell {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            history: PrintHistory::open_default().ok(),
            progress: Box::new(|_| {}),
        }
    }

    /// `copal:<project>$` uses this name.
    pub fn place(&self) -> String {
        self.session
            .path()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".into())
    }

    /// Runs one line and returns what to print.
    pub fn run(&mut self, line: &str) -> Vec<Reply> {
        let mut out = Out::default();
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return out.0;
        }
        let (head, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        let args = split_args(rest);
        match head {
            "help" => self.help(&mut out, rest),
            "commands" => {
                for c in copal_core::registry() {
                    out.info(format!("{:<20} {}", c.name, c.summary));
                }
            }
            "undo" => match self.session.history.undo(&mut self.session.doc) {
                Ok(Some(l)) => out.info(format!("Undo: {l}")),
                Ok(None) => out.info("Nothing to undo"),
                Err(e) => out.err(e),
            },
            "redo" => match self.session.history.redo(&mut self.session.doc) {
                Ok(Some(l)) => out.info(format!("Redo: {l}")),
                Ok(None) => out.info("Nothing to redo"),
                Err(e) => out.err(e),
            },
            "steps" => {
                let labels: Vec<&str> = self.session.history.undo_labels().collect();
                if labels.is_empty() {
                    out.info("No steps yet");
                }
                for (i, l) in labels.iter().enumerate() {
                    out.info(format!("{:>3}  {l}", i + 1));
                }
            }
            "new" => {
                let mut doc = Document::new(if rest.is_empty() { "Untitled" } else { rest });
                doc.printer = self.session.doc.printer.clone();
                doc.resin = self.session.doc.resin.clone();
                doc.settings = self.session.doc.settings;
                self.session = Session::new(doc);
                out.info("New project (printer, resin and settings kept)");
            }
            "open" => match args.first() {
                None => out.err("usage: open <file.copal>"),
                Some(p) => match Session::open(Path::new(p)) {
                    Ok(s) => {
                        self.session = s;
                        out.info(format!("Opened {p}"));
                    }
                    Err(e) => out.err(format!("{p}: {e}")),
                },
            },
            "save" => self.save(&mut out, args.first().map(String::as_str)),
            "import" => {
                if args.is_empty() {
                    out.err("usage: import <file> …");
                }
                for p in &args {
                    self.exec(&mut out, &format!("object.import path={}", quote(p)));
                }
            }
            "export" => {
                self.export(&mut out, args.first().map(String::as_str));
            }
            "info" => self.info(&mut out),
            "supports" => self.supports(&mut out, &args),
            "islands" => self.islands(&mut out, &args),
            "hollow" => {
                let h = self.session.doc.settings.hollow;
                for id in self.targets(&mut out, args.first().map(String::as_str)) {
                    self.exec(
                        &mut out,
                        &format!(
                            "object.hollow id={id} thickness={} voxel={}",
                            h.wall_thickness_mm, h.voxel_mm
                        ),
                    );
                }
            }
            "settings" => self.settings(&mut out, &args),
            "printers" => {
                for p in &library().printers {
                    let mark = if self.session.doc.printer.as_deref() == Some(&p.id) {
                        "*"
                    } else {
                        " "
                    };
                    out.info(format!(
                        "{mark} {:<28} {}  {}×{} px  .{}  {:?}",
                        p.id,
                        p.name(),
                        p.screen.resolution[0],
                        p.screen.resolution[1],
                        p.file.format,
                        p.status
                    ));
                }
            }
            "resins" => {
                let lib = library();
                let printer = self.session.doc.printer.clone().unwrap_or_default();
                let all = args.first().is_some_and(|a| a == "all");
                let mut n = 0;
                for r in &lib.resins {
                    let fits = r.profile_for(&printer).is_some();
                    if !all && !fits {
                        continue;
                    }
                    n += 1;
                    let mark = if self.session.doc.resin.as_deref() == Some(&r.id) {
                        "*"
                    } else {
                        " "
                    };
                    let note = if fits {
                        ""
                    } else {
                        "  (no settings for this printer)"
                    };
                    out.info(format!("{mark} {:<36} {}{note}", r.id, r.title()));
                }
                if n == 0 {
                    out.info("No resins with settings for this printer yet. `resins all` lists every resin; `presets save resin <name>` saves yours.");
                }
            }
            "presets" => self.presets(&mut out, &args),
            "history" => self.history(&mut out, &args),
            _ => self.exec(&mut out, line),
        }
        out.0
    }

    /// Runs a document command.
    pub fn exec(&mut self, out: &mut Out, line: &str) {
        if let Err(e) = self.session.run_line(line) {
            out.err(e);
        }
    }

    fn help(&self, out: &mut Out, rest: &str) {
        if !rest.is_empty() {
            if let Some((_, h)) = COMMANDS.iter().find(|(n, _)| *n == rest) {
                out.info(*h);
            } else if let Some(c) = copal_core::registry().iter().find(|c| c.name == rest) {
                out.info(c.summary);
                out.info(format!("usage: {}", c.usage));
                if let Some(section) = Section::ALL
                    .iter()
                    .find(|s| c.name == format!("{}.set", s.name()))
                {
                    out.info(format!("settings: {}", section.keys().join(", ")));
                }
            } else {
                out.err(format!("no command `{rest}`"));
            }
            return;
        }
        out.info("Shell commands:");
        for (_, h) in COMMANDS {
            out.info(format!("  {h}"));
        }
        out.info("Document commands (`commands` lists them all), for example:");
        out.info(
            "  object.import path=model.stl    object.move id=1 x=10    print.set exposure_s=2.5",
        );
        out.info("`help <command>` shows how to use one command.");
    }

    fn save(&mut self, out: &mut Out, path: Option<&str>) {
        let result = match path {
            Some(p) => self.session.save_as(Path::new(p)).map(|_| true),
            None => self.session.save(),
        };
        match result {
            Ok(true) => out.info(format!(
                "Saved {}",
                self.session
                    .path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            )),
            Ok(false) => out.err("no file yet: use `save <file.copal>`"),
            Err(e) => out.err(e),
        }
    }

    /// Where `export` writes when no file is given: next to the project, or the current folder.
    pub fn default_export_path(&self) -> Result<PathBuf, String> {
        let printer =
            copal_formats::export::project_printer(&self.session.doc).map_err(|e| e.to_string())?;
        let name = copal_formats::export::default_file_name(&self.session.doc, &printer);
        let dir = self
            .session
            .path()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(dir.join(name))
    }

    fn export(&mut self, out: &mut Out, path: Option<&str>) {
        let path = match path {
            Some(p) => PathBuf::from(p),
            None => match self.default_export_path() {
                Ok(p) => p,
                Err(e) => return out.err(e),
            },
        };
        let started = std::time::Instant::now();
        let progress = &self.progress;
        match copal_formats::export::export_and_record(
            &self.session.doc,
            &path,
            &|f| progress(f),
            &AtomicBool::new(false),
            self.history.as_ref(),
        ) {
            Ok((r, record)) => {
                out.info(format!(
                    "Wrote {} — {} layers, {:.1} mm high, {:.1} ml resin, about {} to print (sliced in {:.1} s)",
                    r.path.display(),
                    r.layers,
                    r.height_mm,
                    r.volume_ml,
                    format_duration(r.print_time_s as f64),
                    started.elapsed().as_secs_f32()
                ));
                if let Some(rec) = record {
                    out.info(format!("Saved to the history as {}", rec.id));
                }
            }
            Err(e) => out.err(e),
        }
    }

    fn info(&self, out: &mut Out) {
        let d = &self.session.doc;
        let lib = library();
        out.info(format!("Project  {}", d.name));
        out.info(format!(
            "Printer  {}",
            d.printer
                .as_deref()
                .map(|id| lib
                    .printer(id)
                    .map(|p| p.name())
                    .unwrap_or_else(|| id.to_string()))
                .unwrap_or_else(|| "(not set)".into())
        ));
        out.info(format!(
            "Resin    {}",
            d.resin
                .as_deref()
                .map(|id| lib
                    .resin(id)
                    .map(|r| r.title())
                    .unwrap_or_else(|| id.to_string()))
                .unwrap_or_else(|| "(not set)".into())
        ));
        let e = &d.settings.print.exposure;
        out.info(format!(
            "Layers   {} mm, exposure {} s, bottom {} × {} s",
            e.layer_height_mm, e.exposure_s, e.bottom_layers, e.bottom_exposure_s
        ));
        if let Some(b) = d.bounds() {
            let layers = (b.max.z.max(0.0) / e.layer_height_mm).ceil() as u32;
            out.info(format!(
                "Height   {:.2} mm, {layers} layers, about {} to print",
                b.max.z,
                format_duration(d.settings.print.estimate_time_s(layers) as f64)
            ));
        }
        out.info(format!("Objects  {}", d.objects().len()));
        for o in d.objects() {
            let size = o
                .world_bounds()
                .map(|b| {
                    let s = b.size();
                    format!("{:.2} × {:.2} × {:.2} mm", s.x, s.y, s.z)
                })
                .unwrap_or_else(|| "empty".into());
            out.info(format!(
                "  [{}] {} — {}, {} triangles",
                o.id,
                o.name,
                size,
                o.mesh.triangle_count()
            ));
        }
    }

    fn settings(&self, out: &mut Out, args: &[String]) {
        let sections: Vec<Section> = match args.first() {
            None => Section::ALL.to_vec(),
            Some(name) => match Section::ALL.iter().find(|s| s.name() == name) {
                Some(s) => vec![*s],
                None => {
                    return out.err(format!(
                        "no section `{name}`: use print, supports, hollow or layout"
                    ));
                }
            },
        };
        for s in sections {
            out.info(format!(
                "[{}]  change with: {}.set <setting>=<value>",
                s.name(),
                s.name()
            ));
            for (k, v) in s.table(&self.session.doc.settings) {
                out.info(format!("  {k:<30} {}", copal_core::ops::format_value(&v)));
            }
        }
    }

    // ---------------------------------------------------------------------------- presets

    fn presets(&mut self, out: &mut Out, args: &[String]) {
        let Some(user) = copal_core::user_presets() else {
            return out.err("this system has no config folder for presets");
        };
        let action = args.first().map(String::as_str).unwrap_or("list");
        if action == "list" {
            for kind in PRESET_KINDS {
                let k = Kind::parse(kind).expect("known kind");
                let names = user.list(k);
                out.info(format!(
                    "{kind:<9} {}",
                    if names.is_empty() {
                        "—".to_string()
                    } else {
                        names.join(", ")
                    }
                ));
            }
            out.info(format!("Folder: {}", user.dir.display()));
            if self.session.doc.printer.is_some() {
                out.info("Built-in support presets: light, medium, heavy (`presets load supports medium`)");
            }
            return;
        }
        match action {
            "folder" => {
                return match user.ensure_folder(Kind::Resin) {
                    Ok(dir) => {
                        out.info(format!("Presets: {}", user.dir.display()));
                        out.info(format!(
                            "Resins:  {}  (drop .toml files here, README.txt shows the format)",
                            dir.display()
                        ));
                    }
                    Err(e) => out.err(e),
                };
            }
            "reload" => {
                let problems = copal_core::reload_user_library();
                let n = user.resin_files().iter().filter(|(_, r)| r.is_ok()).count();
                out.info(format!("Personal resins: {n}"));
                for p in problems {
                    out.err(format!("skipped {p}"));
                }
                return;
            }
            "import" => {
                if args.len() < 2 {
                    return out.err("usage: presets import <file.toml>…");
                }
                for file in &args[1..] {
                    match user.import_resin(Path::new(file)) {
                        Ok((r, dest)) => {
                            out.info(format!("Imported {} ({})", r.title(), dest.display()));
                            if let Some(p) = &self.session.doc.printer
                                && r.profile_for(p).is_none()
                            {
                                let printers: Vec<&str> =
                                    r.profiles.iter().map(|p| p.printer.as_str()).collect();
                                out.err(format!(
                                    "{} has no settings for {p}, only for: {}",
                                    r.title(),
                                    printers.join(", ")
                                ));
                            } else {
                                out.info(format!(
                                    "Use it: project.set_resin resin={}",
                                    quote(&r.id)
                                ));
                            }
                        }
                        Err(e) => out.err(e),
                    }
                }
                for p in copal_core::reload_user_library() {
                    out.err(format!("skipped {p}"));
                }
                return;
            }
            _ => {}
        }
        let (Some(kind_name), Some(_)) = (args.get(1), args.get(2)) else {
            return out.err(format!(
                "usage: presets {action} <kind> <name>   kinds: {}",
                PRESET_KINDS.join(", ")
            ));
        };
        let Some(kind) = Kind::parse(kind_name) else {
            return out.err(format!(
                "unknown kind `{kind_name}`: {}",
                PRESET_KINDS.join(", ")
            ));
        };
        // `brand=…`, `color=…` and `type=…` describe a resin; everything else is the name.
        let mut meta = ResinMeta::default();
        let mut words = Vec::new();
        for a in &args[2..] {
            match a.split_once('=') {
                Some(("brand", v)) => meta.brand = v.to_string(),
                Some(("color", v)) => meta.color = v.to_string(),
                Some(("type", v)) => meta.kind = v.to_string(),
                _ => words.push(a.as_str()),
            }
        }
        let name = words.join(" ");
        if name.trim().is_empty() {
            return out.err("give the preset a name");
        }
        let s = self.session.doc.settings;
        match action {
            "save" => {
                let saved = match kind {
                    Kind::Resin => self
                        .resin_preset_with(&name, &meta)
                        .and_then(|r| user.save(kind, &name, &r).map_err(|e| e.to_string())),
                    Kind::Exposure => save(&user, kind, &name, &s.print.exposure),
                    Kind::Motion => save(&user, kind, &name, &s.print.motion),
                    Kind::Supports => save(&user, kind, &name, &s.supports),
                    Kind::Hollow => save(&user, kind, &name, &s.hollow),
                    Kind::Layout => save(&user, kind, &name, &s.layout),
                };
                match saved {
                    Ok(path) => {
                        if kind == Kind::Resin {
                            copal_core::set_library(copal_core::load_user_library());
                        }
                        out.info(format!(
                            "Saved {kind_name} preset `{}` ({})",
                            slug(&name),
                            path.display()
                        ))
                    }
                    Err(e) => out.err(e),
                }
            }
            "load" => {
                let line = match kind {
                    Kind::Resin => {
                        copal_core::set_library(copal_core::load_user_library());
                        Ok(format!(
                            "project.set_resin resin={}",
                            quote(&format!("my/{}", slug(&name)))
                        ))
                    }
                    Kind::Exposure => load_line::<ExposureSettings>(&user, kind, &name, "print"),
                    Kind::Motion => load_line::<MotionSettings>(&user, kind, &name, "print"),
                    Kind::Supports => {
                        match copal_core::presets::builtin_supports()
                            .iter()
                            .find(|(n, _)| *n == name)
                        {
                            Some((_, b)) => Ok(line_from("supports", b)),
                            None => load_line::<SupportSettings>(&user, kind, &name, "supports"),
                        }
                    }
                    Kind::Hollow => load_line::<HollowSettings>(&user, kind, &name, "hollow"),
                    Kind::Layout => load_line::<LayoutSettings>(&user, kind, &name, "layout"),
                };
                match line {
                    Ok(line) => {
                        let before = self.session.history.revision();
                        self.exec(out, &line);
                        if self.session.history.revision() != before {
                            out.info(format!("Loaded {kind_name} preset `{name}`"));
                        }
                    }
                    Err(e) => out.err(e),
                }
            }
            "delete" if kind == Kind::Resin => {
                let id = if name.contains('/') {
                    name.clone()
                } else {
                    format!("my/{}", slug(&name))
                };
                let result = user
                    .delete_resin(&id)
                    .or_else(|_| user.delete(kind, &name).map(|()| PathBuf::from(&name)));
                match result {
                    Ok(path) => {
                        copal_core::reload_user_library();
                        out.info(format!("Deleted resin `{name}` ({})", path.display()))
                    }
                    Err(e) => out.err(e),
                }
            }
            "delete" => match user.delete(kind, &name) {
                Ok(()) => {
                    if kind == Kind::Resin {
                        copal_core::set_library(copal_core::load_user_library());
                    }
                    out.info(format!("Deleted {kind_name} preset `{name}`"))
                }
                Err(e) => out.err(e),
            },
            other => out.err(format!(
                "unknown action `{other}`: list, save, load, delete, import, folder or reload"
            )),
        }
    }

    /// The current print settings as a personal resin for the current printer.
    pub fn resin_preset(&self, name: &str) -> Result<Resin, String> {
        self.resin_preset_with(name, &ResinMeta::default())
    }

    /// Like [`Shell::resin_preset`], with a brand, color and type. Empty ones are taken from the
    /// resin the project uses now (brand: "My").
    pub fn resin_preset_with(&self, name: &str, meta: &ResinMeta) -> Result<Resin, String> {
        let d = &self.session.doc;
        let printer = d.printer.clone().ok_or("choose a printer first")?;
        let base = d
            .resin
            .as_deref()
            .and_then(|id| library().resin(id).cloned());
        let or = |v: &str, d: String| {
            if v.trim().is_empty() {
                d
            } else {
                v.trim().to_string()
            }
        };
        let mut resin = Resin {
            id: format!("my/{}", slug(name)),
            brand: or(&meta.brand, "My".into()),
            name: name.trim().to_string(),
            color: or(&meta.color, String::new()),
            kind: or(
                &meta.kind,
                base.as_ref().map(|b| b.kind.clone()).unwrap_or_default(),
            ),
            based_on: base.as_ref().map(|b| b.id.clone()),
            profiles: Vec::new(),
        };
        // Keep profiles for other printers if this preset existed before.
        if let Some(old) = library().resin(&resin.id) {
            resin.profiles = old.profiles.clone();
        }
        resin.profiles.retain(|p| p.printer != printer);
        resin.profiles.push(ResinProfile {
            printer,
            settings: d.settings.print,
            source: Some(format!("saved by the user, {}", DateTime::now().short())),
        });
        Ok(resin)
    }

    // ---------------------------------------------------------------------------- history

    fn history(&mut self, out: &mut Out, args: &[String]) {
        let Some(h) = self.history.clone() else {
            return out.err("the print history is not available on this system");
        };
        let action = args.first().map(String::as_str).unwrap_or("list");
        let id = args.get(1).map(String::as_str);
        match (action, id) {
            ("list", _) => {
                let list = h.list();
                if list.is_empty() {
                    out.info("No exports yet. `export` slices the project and records it here.");
                }
                for r in list {
                    let stars = r.rating.map(|n| "★".repeat(n as usize)).unwrap_or_default();
                    let outcome = match r.outcome {
                        Some(Outcome::Good) => " ✓",
                        Some(Outcome::Failed) => " ✗",
                        None => "",
                    };
                    out.info(format!(
                        "{}  {}  {} layers, {}{outcome} {stars}",
                        r.id,
                        r.output.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
                        r.layers,
                        format_duration(r.print_time_s as f64),
                    ));
                }
            }
            ("show", Some(id)) => match h.get(id) {
                Ok(r) => {
                    out.info(format!("{}  ({})", r.project_name, r.date));
                    out.info(format!("file      {}", r.output.display()));
                    out.info(format!(
                        "printer   {}   resin {}",
                        r.printer.as_deref().unwrap_or("-"),
                        r.resin.as_deref().unwrap_or("-")
                    ));
                    out.info(format!(
                        "layers    {} × {} mm, {:.1} mm high, {:.1} ml, about {}",
                        r.layers,
                        r.layer_height_mm,
                        r.height_mm,
                        r.volume_ml,
                        format_duration(r.print_time_s as f64)
                    ));
                    out.info(format!(
                        "exposure  {} s, bottom {} × {} s",
                        r.exposure_s, r.bottom_layers, r.bottom_exposure_s
                    ));
                    out.info(format!(
                        "result    {}{}{}",
                        match r.outcome {
                            Some(Outcome::Good) => "printed well",
                            Some(Outcome::Failed) => "failed",
                            None => "not rated",
                        },
                        r.rating.map(|n| format!(", {n}/5")).unwrap_or_default(),
                        if r.note.is_empty() { String::new() } else { format!(" — {}", r.note) }
                    ));
                    out.info(format!("snapshot  {}", h.snapshot_path(id).display()));
                }
                Err(e) => out.err(e),
            },
            ("rate", Some(id)) => {
                let mut r = match h.get(id) {
                    Ok(r) => r,
                    Err(e) => return out.err(e),
                };
                for a in &args[2..] {
                    if let Ok(n) = a.parse::<u8>()
                        && (1..=5).contains(&n)
                    {
                        r.rating = Some(n);
                    } else if let Some(o) = Outcome::parse(a) {
                        r.outcome = Some(o);
                    } else {
                        if !r.note.is_empty() {
                            r.note.push(' ');
                        }
                        r.note.push_str(a);
                    }
                }
                match h.update(&r) {
                    Ok(()) => out.info(format!("Rated {id}")),
                    Err(e) => out.err(e),
                }
            }
            ("open", Some(id)) => match Session::open_copy(&h.snapshot_path(id)) {
                Ok(s) => {
                    self.session = s;
                    out.info(format!(
                        "Opened the project as it was sliced for {id}. Save it with `save <file.copal>`."
                    ));
                }
                Err(e) => out.err(format!("{id}: {e}")),
            },
            ("remove", Some(id)) => match h.remove(id) {
                Ok(()) => out.info(format!("Removed {id} from the history")),
                Err(e) => out.err(e),
            },
            ("folder", _) => out.info(h.dir.display().to_string()),
            _ => out.err("usage: history [list | show <id> | rate <id> [1-5] [good|failed] [note] | open <id> | remove <id> | folder]"),
        }
    }

    /// Object ids from `id`, `all` or nothing (= all).
    fn targets(&self, out: &mut Out, arg: Option<&str>) -> Vec<ObjectId> {
        match arg {
            None | Some("all") => self.object_ids(),
            Some(x) => match x.parse::<u64>() {
                Ok(n) if self.session.doc.object(ObjectId(n)).is_some() => vec![ObjectId(n)],
                _ => {
                    out.err(format!("no object `{x}`"));
                    Vec::new()
                }
            },
        }
    }

    fn supports(&mut self, out: &mut Out, args: &[String]) {
        let action = args.first().map(String::as_str).unwrap_or("list");
        if action == "list" {
            for o in self.session.doc.objects() {
                match &o.prep.supports {
                    Some(s) => out.info(format!(
                        "[{}] {} — {} supports ({} by hand), {} braces{}",
                        o.id,
                        o.name,
                        s.supports.len(),
                        s.supports.iter().filter(|x| x.manual).count(),
                        s.braces.len(),
                        if s.raft.is_empty() { "" } else { ", raft" }
                    )),
                    None => out.info(format!("[{}] {} — no supports", o.id, o.name)),
                }
            }
            return;
        }
        let ids = self.targets(out, args.get(1).map(String::as_str));
        for id in ids {
            let line = match action {
                "auto" => format!("supports.auto id={id}"),
                "islands" => format!("supports.auto id={id} mode=islands"),
                "clear" => format!("supports.clear id={id}"),
                other => return out.err(format!("unknown action `{other}`: auto, islands, clear")),
            };
            self.exec(out, &line);
            if let Some(s) = self
                .session
                .doc
                .object(id)
                .and_then(|o| o.prep.supports.clone())
            {
                out.info(format!(
                    "[{id}] {} supports, {} braces",
                    s.supports.len(),
                    s.braces.len()
                ));
            }
        }
    }

    fn islands(&mut self, out: &mut Out, args: &[String]) {
        for id in self.targets(out, args.first().map(String::as_str)) {
            match copal_core::prepare::unsupported_islands(&self.session.doc, id) {
                Ok(list) if list.is_empty() => {
                    out.info(format!("[{id}] every island is supported"))
                }
                Ok(list) => {
                    out.err(format!("[{id}] {} islands without support:", list.len()));
                    for i in list.iter().take(20) {
                        let c = (i.min + i.max) / 2.0;
                        out.info(format!(
                            "    at x={:.1} y={:.1} z={:.2}, {:.2} mm²",
                            c.x, c.y, i.z, i.area
                        ));
                    }
                }
                Err(e) => out.err(e),
            }
        }
    }

    /// Completes the word before the cursor: shell commands, document commands, then argument
    /// names of the command on the line.
    pub fn complete(&self, line: &str) -> Vec<String> {
        let (head, rest) = match line.split_once(' ') {
            Some((h, r)) => (h, Some(r)),
            None => (line, None),
        };
        match rest {
            None => {
                let mut names: Vec<String> = COMMANDS
                    .iter()
                    .map(|(n, _)| n.to_string())
                    .chain(copal_core::registry().iter().map(|c| c.name.to_string()))
                    .filter(|n| n.starts_with(head))
                    .collect();
                names.sort();
                names
            }
            Some(rest) => {
                let word = rest.rsplit(' ').next().unwrap_or("");
                let keys: Vec<String> = if let Some(section) = Section::ALL
                    .iter()
                    .find(|s| head == format!("{}.set", s.name()))
                {
                    section
                        .keys()
                        .into_iter()
                        .map(|k| format!("{k}="))
                        .collect()
                } else if head == "settings" {
                    Section::ALL.iter().map(|s| s.name().to_string()).collect()
                } else if head == "presets" {
                    [
                        "save", "load", "delete", "list", "import", "folder", "reload",
                    ]
                    .iter()
                    .map(|s| s.to_string())
                    .chain(PRESET_KINDS.iter().map(|s| s.to_string()))
                    .collect()
                } else if head == "history" {
                    ["show", "rate", "open", "remove", "list", "folder"]
                        .iter()
                        .map(|s| s.to_string())
                        .chain(
                            self.history
                                .iter()
                                .flat_map(|h| h.list().into_iter().map(|r| r.id)),
                        )
                        .collect()
                } else if head == "project.set_printer" {
                    library()
                        .printers
                        .iter()
                        .map(|p| format!("printer={}", p.id))
                        .collect()
                } else if head == "project.set_resin" {
                    library()
                        .resins
                        .iter()
                        .map(|r| format!("resin={}", r.id))
                        .collect()
                } else if let Some(c) = copal_core::registry().iter().find(|c| c.name == head) {
                    usage_keys(c.usage)
                } else {
                    Vec::new()
                };
                keys.into_iter().filter(|k| k.starts_with(word)).collect()
            }
        }
    }

    /// The ids of the objects, for commands that take `id=`.
    pub fn object_ids(&self) -> Vec<ObjectId> {
        self.session.doc.objects().iter().map(|o| o.id).collect()
    }
}

/// `key=` names mentioned in a usage string.
fn usage_keys(usage: &str) -> Vec<String> {
    let mut keys: Vec<String> = usage
        .split(|c: char| c.is_whitespace() || c == '[' || c == ']' || c == '|')
        .filter_map(|w| w.split_once('=').map(|(k, _)| format!("{k}=")))
        .filter(|k| k.len() > 1)
        .collect();
    keys.dedup();
    keys
}

fn save<T: Serialize>(
    user: &copal_core::presets::UserPresets,
    kind: Kind,
    name: &str,
    v: &T,
) -> Result<PathBuf, String> {
    user.save(kind, name, v).map_err(|e| e.to_string())
}

fn line_from<T: Serialize>(section: &str, v: &T) -> String {
    let table = toml::Table::try_from(v).unwrap_or_default();
    let mut line = format!("{section}.set");
    for (k, v) in table {
        line.push_str(&format!(" {k}={}", copal_core::ops::format_value(&v)));
    }
    line
}

fn load_line<T: Serialize + serde::de::DeserializeOwned>(
    user: &copal_core::presets::UserPresets,
    kind: Kind,
    name: &str,
    section: &str,
) -> Result<String, String> {
    let v: T = user.load(kind, name).map_err(|e| e.to_string())?;
    Ok(line_from(section, &v))
}

/// Splits arguments on spaces, keeping "quoted parts" together.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// Collects replies.
#[derive(Debug, Default)]
pub struct Out(pub Vec<Reply>);

impl Out {
    pub fn info(&mut self, s: impl Into<String>) {
        self.0.push(Reply::Info(s.into()));
    }
    pub fn err(&mut self, e: impl ToString) {
        self.0.push(Reply::Error(e.to_string()));
    }
}

/// Keeps `SetSettings` reachable for apps that build preset lines themselves.
pub fn settings_line(section: Section, doc: &Document) -> String {
    SetSettings::line_for(section, &doc.settings)
}

/// Brand, color and type of a resin saved from the current settings.
#[derive(Debug, Clone, Default)]
pub struct ResinMeta {
    pub brand: String,
    pub color: String,
    pub kind: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(dir: &Path) -> Shell {
        // The history goes to a temporary folder, away from the user's real one.
        let mut s = Shell::new(Session::new(Document::new("t")));
        s.history = Some(PrintHistory::new(dir.join("history")));
        s
    }

    fn text(r: &[Reply]) -> String {
        r.iter().map(Reply::text).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn document_and_shell_commands() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = shell(dir.path());
        assert!(text(&s.run("help")).contains("export"));
        assert!(text(&s.run("help print.set")).contains("exposure_s"));
        s.run("project.set_printer printer=elegoo/saturn-4-ultra-16k");
        s.run("object.add_cube size=4");
        assert_eq!(s.session.doc.objects().len(), 1);
        assert!(text(&s.run("steps")).contains("object.add_cube size=4"));
        assert_eq!(
            s.run("undo"),
            vec![Reply::Info("Undo: object.add_cube size=4".into())]
        );
        s.run("redo");
        assert!(text(&s.run("info")).contains("4.00 × 4.00 × 4.00 mm"));
        assert!(text(&s.run("settings print")).contains("exposure_s"));
        assert!(matches!(s.run("object.move id=9 x=1")[0], Reply::Error(_)));
        assert!(text(&s.run("printers")).contains("* elegoo/saturn-4-ultra-16k"));
    }

    #[test]
    fn export_records_history_and_reopens_the_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = shell(dir.path());
        s.run("project.set_printer printer=elegoo/saturn-4-ultra-16k");
        s.run("print.set layer_height_mm=0.1");
        s.run("object.add_cube size=3");
        let goo = dir.path().join("cube.goo");
        let r = s.run(&format!("export {}", quote(&goo.to_string_lossy())));
        assert!(text(&r).contains("30 layers"), "{}", text(&r));
        assert!(goo.exists());
        let id = s.history.as_ref().unwrap().list()[0].id.clone();
        s.run(&format!("history rate {id} 4 good no stringing"));
        let shown = text(&s.run(&format!("history show {id}")));
        assert!(
            shown.contains("printed well, 4/5 — no stringing"),
            "{shown}"
        );
        s.run("object.remove id=1");
        assert!(s.session.doc.objects().is_empty());
        s.run(&format!("history open {id}"));
        assert_eq!(s.session.doc.objects().len(), 1);
        assert!(s.session.is_dirty() && s.session.path().is_none());
    }

    #[test]
    fn completion() {
        let dir = tempfile::tempdir().unwrap();
        let s = shell(dir.path());
        assert!(s.complete("expo").contains(&"export".to_string()));
        assert!(
            s.complete("object.im")
                .contains(&"object.import".to_string())
        );
        assert!(
            s.complete("print.set expo")
                .contains(&"exposure_s=".to_string())
        );
        assert!(s.complete("object.move id=1 ").contains(&"x=".to_string()));
        assert_eq!(split_args(r#"a "b c" d"#), vec!["a", "b c", "d"]);
    }
}
