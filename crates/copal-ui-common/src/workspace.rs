//! State shared by Copal and Copal Lite: the open project, selection, camera, console, file
//! dialogs and background export. The apps only decide the layout; the panels are in
//! [`crate::panels`] and the 3D view in [`crate::view3d`].

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use copal_core::prints::PrintRecord;
use copal_core::{Document, ObjectId, Session, Settings, Transform, script::quote};
use copal_formats::{ExportError, ExportReport};
use copal_shell::{Reply, Shell};
use egui::{Key, KeyboardShortcut, Modifiers, Ui};

use crate::camera::Camera;
use crate::files::{FileDialog, Purpose};
use crate::gizmo;
use crate::render::Renderer;
use crate::view3d::{LayerPreview, ViewDrag};

/// The three workflow steps, like the big tabs in Lychee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Place, rotate, scale, copy and arrange models.
    Layout,
    /// Supports, hollowing, holes, checks.
    Prepare,
    /// Slice, preview layers, write the printer file.
    Export,
}

impl Step {
    pub const ALL: [(Step, &'static str); 3] = [
        (Step::Layout, "Layout"),
        (Step::Prepare, "Prepare"),
        (Step::Export, "Export"),
    ];
}

/// Tabs of the right panel: one per kind of settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Object,
    Resin,
    Print,
    Layout,
    Supports,
    Hollow,
}

impl SettingsTab {
    pub const ALL: [(SettingsTab, &'static str); 6] = [
        (SettingsTab::Object, "Object"),
        (SettingsTab::Resin, "Resin"),
        (SettingsTab::Print, "Print"),
        (SettingsTab::Layout, "Layout"),
        (SettingsTab::Supports, "Supports"),
        (SettingsTab::Hollow, "Hollow"),
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// A command the user typed, with the prompt it was typed at.
    Input {
        prompt: String,
    },
    Info,
    Error,
}

/// Console commands that only the apps have (the shell has the rest).
pub const APP_COMMANDS: &[&str] = &["clear", "preferences"];

type ExportResult = Result<(ExportReport, Option<PrintRecord>), ExportError>;

/// An export running in the background.
pub struct ExportJob {
    pub path: PathBuf,
    pub progress: Arc<AtomicU32>,
    pub cancel: Arc<AtomicBool>,
    handle: Option<JoinHandle<ExportResult>>,
    pub started: std::time::Instant,
}

impl ExportJob {
    /// 0..1
    pub fn fraction(&self) -> f32 {
        self.progress.load(Ordering::Relaxed) as f32 / 10_000.0
    }
}

pub struct Workspace {
    pub shell: Shell,
    pub settings: Settings,
    pub selected: Option<ObjectId>,
    pub camera: Camera,
    pub step: Step,
    pub tab: SettingsTab,
    pub gizmo_mode: gizmo::Mode,
    /// Color faces that need supports.
    pub show_overhangs: bool,
    pub show_settings: bool,
    pub show_history: bool,
    pub console_input: String,
    pub console: Vec<(LineKind, String)>,
    settings_changed: bool,
    /// Typed commands, oldest first, for ↑ / ↓.
    pub(crate) typed: Vec<String>,
    pub(crate) typed_pos: Option<usize>,
    pub(crate) console_focused: bool,
    /// Transform being edited in the inspector: (object, value before editing started).
    pub(crate) editing: Option<(ObjectId, Transform)>,
    /// Values of settings fields while they are dragged (committed on release).
    pub(crate) field_edit: std::collections::HashMap<String, f64>,
    pub(crate) drag: Option<ViewDrag>,
    pub(crate) hot_handle: Option<gizmo::Handle>,
    pub(crate) renderer: Arc<Mutex<Option<Result<Renderer, String>>>>,
    pub(crate) dialog: Option<FileDialog>,
    pub(crate) last_dir: Option<PathBuf>,
    pub export_job: Option<ExportJob>,
    pub last_export: Option<Result<ExportReport, String>>,
    pub(crate) preview: LayerPreview,
    pub(crate) history_textures: std::collections::HashMap<String, egui::TextureHandle>,
    pub(crate) history_note: std::collections::HashMap<String, String>,
    pub(crate) framed: bool,
    pub(crate) preset_name: String,
    pub(crate) resins: crate::resins::ResinState,
    pub(crate) prep: crate::prep::PrepState,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

/// The printer new projects start with.
pub const DEFAULT_PRINTER: &str = "elegoo/saturn-4-ultra-16k";

impl Workspace {
    pub fn new(settings: Settings) -> Self {
        copal_core::set_library(copal_core::load_user_library());
        let mut session = Session::new(Document::new("Untitled"));
        let _ = session.run_line(&format!("project.set_printer printer={DEFAULT_PRINTER}"));
        // A fresh project is not "changed".
        let session = Session::new(session.doc.clone());
        let mut ws = Self {
            shell: Shell::new(session),
            settings,
            selected: None,
            camera: Camera::default(),
            step: Step::Layout,
            tab: SettingsTab::Object,
            gizmo_mode: gizmo::Mode::Move,
            show_overhangs: false,
            show_settings: false,
            show_history: false,
            console_input: String::new(),
            console: Vec::new(),
            settings_changed: false,
            typed: Vec::new(),
            typed_pos: None,
            console_focused: false,
            editing: None,
            field_edit: Default::default(),
            drag: None,
            hot_handle: None,
            renderer: Arc::new(Mutex::new(None)),
            dialog: None,
            last_dir: None,
            export_job: None,
            last_export: None,
            preview: LayerPreview::default(),
            history_textures: Default::default(),
            history_note: Default::default(),
            framed: false,
            preset_name: String::new(),
            resins: Default::default(),
            prep: Default::default(),
        };
        ws.log(
            LineKind::Info,
            format!(
                "Copal {} console. Type `help`, press Tab to complete.",
                copal_core::VERSION
            ),
        );
        ws
    }

    pub fn session(&self) -> &Session {
        &self.shell.session
    }

    pub fn doc(&self) -> &Document {
        &self.shell.session.doc
    }

    /// Opens a project file.
    pub fn open(&mut self, path: &Path) {
        match Session::open(path) {
            Ok(s) => {
                self.shell.session = s;
                self.after_document_swap();
                self.log(LineKind::Info, format!("Opened {}", path.display()));
            }
            Err(e) => self.log(LineKind::Error, format!("{}: {e}", path.display())),
        }
    }

    fn after_document_swap(&mut self) {
        self.selected = None;
        self.editing = None;
        self.drag = None;
        self.framed = false;
        self.preview = LayerPreview::default();
        self.prep.selection.clear();
        self.prep.islands = None;
        self.prep.cache.clear();
    }

    /// True once after the settings changed; the app then re-applies its look.
    pub fn take_settings_changed(&mut self) -> bool {
        std::mem::take(&mut self.settings_changed)
    }

    pub fn log(&mut self, kind: LineKind, text: impl Into<String>) {
        self.console.push((kind, text.into()));
        if self.console.len() > 1000 {
            self.console.drain(..200);
        }
    }

    pub fn prompt_place(&self) -> String {
        self.shell.place()
    }

    /// Runs one console line: an app command, a shell command or a document command.
    pub fn run(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        let prompt = self.prompt_place();
        self.log(LineKind::Input { prompt }, line);
        self.run_quiet(line);
    }

    /// Like [`run`](Self::run), without echoing the line (for buttons that already show what
    /// they do). Errors and messages still go to the console.
    pub fn run_quiet(&mut self, line: &str) {
        let (head, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        match head {
            "clear" => {
                self.console.clear();
                return;
            }
            "preferences" => {
                self.show_settings = true;
                return;
            }
            "export" => {
                let path = if rest.is_empty() {
                    self.shell.default_export_path().map_err(|e| e.to_string())
                } else {
                    Ok(PathBuf::from(rest.trim_matches('"')))
                };
                match path {
                    Ok(p) => self.start_export(p),
                    Err(e) => self.log(LineKind::Error, e),
                }
                return;
            }
            _ => {}
        }
        let before: Vec<ObjectId> = self.doc().objects().iter().map(|o| o.id).collect();
        let doc_before = self.shell.session.path().map(Path::to_path_buf);
        let objects_before = self.doc().objects().len();
        for r in self.shell.run(line) {
            match r {
                Reply::Info(t) => self.log(LineKind::Info, t),
                Reply::Error(t) => self.log(LineKind::Error, t),
            }
        }
        if matches!(head, "open" | "new") || (head == "history" && rest.starts_with("open")) {
            self.after_document_swap();
        }
        // Select a newly added object.
        let new: Vec<ObjectId> = self
            .doc()
            .objects()
            .iter()
            .map(|o| o.id)
            .filter(|id| !before.contains(id))
            .collect();
        if let Some(last) = new.last() {
            self.selected = Some(*last);
        }
        if objects_before == 0 && !new.is_empty() {
            self.framed = false;
        }
        let _ = doc_before;
        self.fix_selection();
    }

    /// Runs a document command; on error logs it. Returns true if it worked.
    pub fn exec(&mut self, line: &str) -> bool {
        let prompt = self.prompt_place();
        self.log(LineKind::Input { prompt }, line);
        match self.shell.session.run_line(line) {
            Ok(()) => {
                self.fix_selection();
                true
            }
            Err(e) => {
                self.log(LineKind::Error, e.to_string());
                false
            }
        }
    }

    /// Applies and saves new settings.
    pub fn set_settings(&mut self, settings: Settings) {
        if settings == self.settings {
            return;
        }
        self.settings = settings;
        self.settings_changed = true;
        if let Err(e) = self.settings.save() {
            self.log(LineKind::Error, format!("could not save settings: {e}"));
        }
    }

    /// Opens or folds the side windows (saved with the app settings).
    pub fn set_panels(&mut self, panels: copal_core::Panels) {
        if panels != self.settings.panels {
            let mut s = self.settings.clone();
            s.panels = panels;
            self.set_settings(s);
        }
    }

    pub fn undo(&mut self) {
        self.run("undo");
    }

    pub fn redo(&mut self) {
        self.run("redo");
    }

    pub(crate) fn fix_selection(&mut self) {
        if self
            .selected
            .is_some_and(|id| self.doc().object(id).is_none())
        {
            self.selected = None;
        }
    }

    /// Opens a file dialog.
    pub fn open_dialog(&mut self, purpose: Purpose) {
        let ext = copal_formats::export::project_printer(self.doc())
            .map(|p| p.file.format)
            .unwrap_or_else(|_| "goo".into());
        let name = match purpose {
            Purpose::SaveProject => format!("{}.copal", self.doc().name),
            Purpose::Export => self
                .shell
                .default_export_path()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_default(),
            _ => String::new(),
        };
        let start = self.last_dir.clone().or_else(|| {
            self.session()
                .path()
                .and_then(|p| p.parent())
                .map(Path::to_path_buf)
        });
        self.dialog = Some(FileDialog::new(purpose, start.as_deref(), &name, &ext));
    }

    /// Draws the open file dialog and acts on the result.
    pub fn dialogs(&mut self, ctx: &egui::Context, look: &crate::Look) {
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        let Some(paths) = dialog.show(ctx, look) else {
            return;
        };
        let purpose = dialog.purpose;
        self.last_dir = Some(dialog.dir().to_path_buf());
        self.dialog = None;
        let Some(first) = paths.first().cloned() else {
            return;
        };
        match purpose {
            Purpose::ImportModel => self.import(&paths),
            Purpose::OpenProject => self.open(&first),
            Purpose::SaveProject => self.run(&format!("save {}", quote(&first.to_string_lossy()))),
            Purpose::Export => self.start_export(first),
            Purpose::ImportResin => self.import_resins(&paths),
        }
    }

    pub fn import(&mut self, paths: &[PathBuf]) {
        let mut line = String::from("import");
        for p in paths {
            line.push(' ');
            line.push_str(&quote(&p.to_string_lossy()));
        }
        self.step = Step::Layout;
        self.run(&line);
    }

    /// Files dropped on the window: models are imported, a project is opened, resin presets
    /// (`.toml`) go to the personal resin folder.
    pub fn dropped_files(&mut self, ui: &Ui) {
        let dropped: Vec<PathBuf> = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if dropped.is_empty() {
            return;
        }
        let has =
            |p: &PathBuf, ext: &str| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext));
        let (presets, dropped): (Vec<PathBuf>, Vec<PathBuf>) =
            dropped.into_iter().partition(|p| has(p, "toml"));
        if !presets.is_empty() {
            self.import_resins(&presets);
        }
        let (projects, models): (Vec<PathBuf>, Vec<PathBuf>) =
            dropped.into_iter().partition(|p| has(p, "copal"));
        if let Some(p) = projects.first() {
            self.open(p);
        }
        if !models.is_empty() {
            self.import(&models);
        }
    }

    /// Starts slicing in the background.
    pub fn start_export(&mut self, path: PathBuf) {
        if self.export_job.is_some() {
            self.log(LineKind::Error, "an export is already running");
            return;
        }
        let doc = self.doc().clone();
        if let Err(e) = copal_formats::export::project_printer(&doc)
            .and_then(|p| copal_formats::export::check_fits(&doc, &p))
        {
            self.log(LineKind::Error, e.to_string());
            self.last_export = Some(Err(e.to_string()));
            return;
        }
        let progress = Arc::new(AtomicU32::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let history = self.shell.history.clone();
        let (p2, c2, path2) = (progress.clone(), cancel.clone(), path.clone());
        self.log(LineKind::Info, format!("Slicing to {}…", path.display()));
        let handle = std::thread::spawn(move || {
            copal_formats::export::export_and_record(
                &doc,
                &path2,
                &|f| p2.store((f * 10_000.0) as u32, Ordering::Relaxed),
                &c2,
                history.as_ref(),
            )
        });
        self.export_job = Some(ExportJob {
            path,
            progress,
            cancel,
            handle: Some(handle),
            started: std::time::Instant::now(),
        });
        self.step = Step::Export;
    }

    /// Checks on the background export. Call every frame.
    pub fn poll_export(&mut self, ctx: &egui::Context) {
        let Some(job) = &mut self.export_job else {
            return;
        };
        if !job.handle.as_ref().is_some_and(|h| h.is_finished()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        let secs = job.started.elapsed().as_secs_f32();
        let result = job.handle.take().map(|h| h.join());
        self.export_job = None;
        match result {
            Some(Ok(Ok((report, record)))) => {
                self.log(
                    LineKind::Info,
                    format!(
                        "Wrote {} — {} layers, {:.1} ml, about {} to print (sliced in {secs:.1} s)",
                        report.path.display(),
                        report.layers,
                        report.volume_ml,
                        copal_core::time::format_duration(report.print_time_s as f64)
                    ),
                );
                if let Some(r) = record {
                    self.log(
                        LineKind::Info,
                        format!("Saved to the print history as {}", r.id),
                    );
                }
                self.last_export = Some(Ok(report));
            }
            Some(Ok(Err(e))) => {
                self.log(LineKind::Error, e.to_string());
                self.last_export = Some(Err(e.to_string()));
            }
            _ => {
                self.log(LineKind::Error, "the export stopped unexpectedly");
                self.last_export = Some(Err("the export stopped unexpectedly".into()));
            }
        }
    }

    /// Global hotkeys. Call once per frame before drawing panels.
    pub fn hotkeys(&mut self, ui: &Ui) {
        let ctx = ui.ctx();
        let shortcut = |m, k| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        let redo = shortcut(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
            || shortcut(Modifiers::COMMAND, Key::Y);
        let undo = shortcut(Modifiers::COMMAND, Key::Z);
        let save_as = shortcut(Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
        let save = shortcut(Modifiers::COMMAND, Key::S);
        let open = shortcut(Modifiers::COMMAND, Key::O);
        let import = shortcut(Modifiers::COMMAND, Key::I);
        let export = shortcut(Modifiers::COMMAND, Key::E);
        let dup = shortcut(Modifiers::COMMAND, Key::D);
        let prefs = shortcut(Modifiers::COMMAND, Key::Comma);
        let alt =
            |k| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::ALT, k)));
        let mut panels = self.settings.panels;
        if alt(Key::Num1) {
            panels.tools = !panels.tools;
        }
        if alt(Key::Num2) {
            panels.settings = !panels.settings;
        }
        if alt(Key::Num3) || alt(Key::F12) {
            panels.console = !panels.console;
        }
        self.set_panels(panels);
        let typing = ctx.memory(|m| m.focused().is_some()) || self.dialog.is_some();
        let key = |k| !typing && ctx.input(|i| i.key_pressed(k) && i.modifiers.is_none());
        if redo {
            self.redo();
        } else if undo {
            self.undo();
        }
        if save_as || (save && self.session().path().is_none()) {
            self.open_dialog(Purpose::SaveProject);
        } else if save {
            self.run("save");
        }
        if open {
            self.open_dialog(Purpose::OpenProject);
        }
        if import {
            self.open_dialog(Purpose::ImportModel);
        }
        if export {
            self.open_dialog(Purpose::Export);
        }
        if prefs {
            self.show_settings = !self.show_settings;
        }
        let delete = key(Key::Delete) || key(Key::X);
        if self.step == Step::Prepare && !self.prep.selection.is_empty() {
            if delete {
                self.delete_selected_supports();
            }
        } else if let Some(id) = self.selected {
            if dup && !typing {
                self.run(&format!("object.duplicate id={id}"));
            }
            if delete {
                self.run(&format!("object.remove id={id}"));
            }
        }
        // Blender keys pick the gizmo.
        if key(Key::G) {
            self.gizmo_mode = gizmo::Mode::Move;
            self.step = Step::Layout;
        }
        if key(Key::R) {
            self.gizmo_mode = gizmo::Mode::Rotate;
            self.step = Step::Layout;
        }
        if key(Key::S) {
            self.gizmo_mode = gizmo::Mode::Scale;
            self.step = Step::Layout;
        }
        if key(Key::Escape) {
            if let Some(job) = &self.export_job {
                job.cancel.store(true, Ordering::Relaxed);
            } else if !self.prep.selection.is_empty() {
                self.prep.selection.clear();
            } else if self.prep.tool != crate::prep::PrepTool::Look {
                self.prep.tool = crate::prep::PrepTool::Look;
            } else {
                self.selected = None;
            }
        }
        if key(Key::F) {
            self.framed = false;
        }
    }

    pub fn title(&self) -> String {
        let dirty = if self.session().is_dirty() {
            " •"
        } else {
            ""
        };
        format!("{}{dirty} — Copal", self.doc().name)
    }

    /// Live preview of a transform while the user drags (not recorded).
    pub(crate) fn preview_transform(&mut self, id: ObjectId, t: Transform) {
        if let Some(o) = self.shell.session.doc.object_mut(id) {
            o.transform = t;
        }
    }

    /// Ends a live edit: puts the old transform back and records the new one as one step.
    pub(crate) fn commit_transform(&mut self, id: ObjectId, old: Transform) {
        let Some(o) = self.shell.session.doc.object_mut(id) else {
            return;
        };
        let new = std::mem::replace(&mut o.transform, old);
        if new == old {
            return;
        }
        let (p, r, s) = (new.translation, new.rotation_deg, new.scale);
        self.exec(&format!(
            "object.set_transform id={id} tx={} ty={} tz={} rx={} ry={} rz={} sx={} sy={} sz={}",
            p.x, p.y, p.z, r.x, r.y, r.z, s.x, s.y, s.z
        ));
    }
}
