//! A simple file chooser drawn with egui. It works everywhere Copal runs, also without a desktop
//! (Copal Lite in a kiosk session), so we don't depend on system dialogs.

use std::path::{Path, PathBuf};

use egui::{Context, Key, RichText, ScrollArea, TextEdit};

use crate::look::Look;
use crate::widgets::{self, ButtonKind};

/// What the chosen file is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    ImportModel,
    OpenProject,
    SaveProject,
    Export,
    /// A resin preset file (`.toml`) to copy into the personal resin folder.
    ImportResin,
}

impl Purpose {
    fn title(self) -> &'static str {
        match self {
            Purpose::ImportModel => "Import models",
            Purpose::OpenProject => "Open project",
            Purpose::SaveProject => "Save project",
            Purpose::Export => "Export printer file",
            Purpose::ImportResin => "Import resin preset",
        }
    }

    fn saving(self) -> bool {
        matches!(self, Purpose::SaveProject | Purpose::Export)
    }

    fn extensions(self, export_ext: &str) -> Vec<String> {
        match self {
            Purpose::ImportModel => vec!["stl".into(), "obj".into(), "3mf".into()],
            Purpose::OpenProject | Purpose::SaveProject => vec!["copal".into()],
            Purpose::Export => vec![export_ext.to_string()],
            Purpose::ImportResin => vec!["toml".into()],
        }
    }
}

struct Entry {
    name: String,
    path: PathBuf,
    dir: bool,
    size: u64,
}

pub struct FileDialog {
    pub purpose: Purpose,
    dir: PathBuf,
    name: String,
    extensions: Vec<String>,
    entries: Vec<Entry>,
    selected: Vec<PathBuf>,
    path_text: String,
    error: Option<String>,
}

/// Folders worth a shortcut button.
fn places() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    if let Some(h) = home {
        out.push(("Home".to_string(), h.clone()));
        for (label, sub) in [
            ("Desktop", "Desktop"),
            ("Downloads", "Downloads"),
            ("Documents", "Documents"),
            ("Рабочий стол", "Рабочий стол"),
            ("Загрузки", "Загрузки"),
            ("Документы", "Документы"),
        ] {
            let p = h.join(sub);
            if p.is_dir() {
                out.push((label.to_string(), p));
            }
        }
    }
    if cfg!(unix) {
        for p in ["/media", "/mnt", "/run/media"] {
            if Path::new(p).is_dir() {
                out.push((p.to_string(), PathBuf::from(p)));
            }
        }
    }
    out
}

fn human_size(b: u64) -> String {
    match b {
        0..1024 => format!("{b} B"),
        1024..1_048_576 => format!("{:.0} KB", b as f64 / 1024.0),
        1_048_576..1_073_741_824 => format!("{:.1} MB", b as f64 / 1_048_576.0),
        _ => format!("{:.2} GB", b as f64 / 1_073_741_824.0),
    }
}

impl FileDialog {
    /// `start` is the folder to show first; `name` the suggested file name when saving.
    pub fn new(purpose: Purpose, start: Option<&Path>, name: &str, export_ext: &str) -> Self {
        let dir = start
            .filter(|p| p.is_dir())
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/"));
        let mut d = Self {
            purpose,
            dir: dir.clone(),
            name: name.to_string(),
            extensions: purpose.extensions(export_ext),
            entries: Vec::new(),
            selected: Vec::new(),
            path_text: dir.display().to_string(),
            error: None,
        };
        d.refresh();
        d
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.error = None;
        match std::fs::read_dir(&self.dir) {
            Ok(rd) => {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') {
                        continue;
                    }
                    let path = e.path();
                    let dir = path.is_dir();
                    let ext_ok = path
                        .extension()
                        .map(|x| x.to_string_lossy().to_lowercase())
                        .is_some_and(|x| self.extensions.contains(&x));
                    if dir || ext_ok {
                        let size = if dir {
                            0
                        } else {
                            e.metadata().map(|m| m.len()).unwrap_or(0)
                        };
                        self.entries.push(Entry {
                            name,
                            path,
                            dir,
                            size,
                        });
                    }
                }
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        self.entries.sort_by(|a, b| {
            b.dir
                .cmp(&a.dir)
                .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.path_text = self.dir.display().to_string();
    }

    fn go(&mut self, dir: PathBuf) {
        self.dir = dir;
        self.selected.clear();
        self.refresh();
    }

    fn save_path(&self) -> Option<PathBuf> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        let mut p = self.dir.join(name);
        let ext = &self.extensions[0];
        if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case(ext)) {
            p.set_extension(ext);
        }
        Some(p)
    }

    /// Shows the dialog. Returns the chosen files (one when saving), or `Some(vec![])` when
    /// cancelled. `None` while still open.
    pub fn show(&mut self, ctx: &Context, look: &Look) -> Option<Vec<PathBuf>> {
        let mut result = None;
        let mut open = true;
        egui::Window::new(self.purpose.title())
            .open(&mut open)
            .collapsible(false)
            .default_size([620.0, 460.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if widgets::button(ui, look, ButtonKind::Normal, "Up").clicked()
                        && let Some(parent) = self.dir.parent()
                    {
                        let parent = parent.to_path_buf();
                        self.go(parent);
                    }
                    let r = ui.add(
                        TextEdit::singleline(&mut self.path_text).desired_width(f32::INFINITY),
                    );
                    if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        let p = PathBuf::from(self.path_text.trim());
                        if p.is_dir() {
                            self.go(p);
                        } else {
                            self.error = Some("no such folder".into());
                        }
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    for (label, p) in places() {
                        if ui.small_button(label).clicked() {
                            self.go(p);
                        }
                    }
                });
                ui.separator();
                let list_height =
                    ui.available_height() - if self.purpose.saving() { 80.0 } else { 46.0 };
                let mut go_to = None;
                let mut pick = None;
                ScrollArea::vertical()
                    .max_height(list_height.max(120.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(e) = &self.error {
                            ui.label(RichText::new(e).color(look.pal.error));
                        }
                        if self.entries.is_empty() && self.error.is_none() {
                            ui.label(
                                RichText::new(format!(
                                    "No folders or .{} files here",
                                    self.extensions.join(" / .")
                                ))
                                .color(look.pal.text_faint),
                            );
                        }
                        for e in &self.entries {
                            let selected = self.selected.contains(&e.path);
                            let label = if e.dir {
                                format!("▸  {}", e.name)
                            } else {
                                format!("    {}    {}", e.name, human_size(e.size))
                            };
                            let r = ui.selectable_label(selected, label);
                            if r.double_clicked() {
                                if e.dir {
                                    go_to = Some(e.path.clone());
                                } else {
                                    pick = Some(e.path.clone());
                                }
                            } else if r.clicked() {
                                if e.dir {
                                    go_to = Some(e.path.clone());
                                } else {
                                    let multi =
                                        ui.input(|i| i.modifiers.command || i.modifiers.shift);
                                    if !multi
                                        || !matches!(
                                            self.purpose,
                                            Purpose::ImportModel | Purpose::ImportResin
                                        )
                                    {
                                        self.selected.clear();
                                    }
                                    if selected && multi {
                                        self.selected.retain(|p| p != &e.path);
                                    } else {
                                        self.selected.push(e.path.clone());
                                    }
                                    if self.purpose.saving() {
                                        self.name = e.name.clone();
                                    }
                                }
                            }
                        }
                    });
                if let Some(d) = go_to {
                    self.go(d);
                }
                if let Some(p) = pick {
                    result = Some(if self.purpose.saving() {
                        vec![p]
                    } else {
                        let mut v = self.selected.clone();
                        if !v.contains(&p) {
                            v = vec![p];
                        }
                        v
                    });
                }
                ui.separator();
                if self.purpose.saving() {
                    ui.horizontal(|ui| {
                        ui.label("File name");
                        ui.add(TextEdit::singleline(&mut self.name).desired_width(f32::INFINITY));
                    });
                }
                ui.horizontal(|ui| {
                    let label = match self.purpose {
                        Purpose::ImportModel => "Import",
                        Purpose::OpenProject => "Open",
                        Purpose::SaveProject => "Save",
                        Purpose::Export => "Export",
                        Purpose::ImportResin => "Import",
                    };
                    let ready = if self.purpose.saving() {
                        self.save_path().is_some()
                    } else {
                        !self.selected.is_empty()
                    };
                    ui.add_enabled_ui(ready, |ui| {
                        if widgets::button(ui, look, ButtonKind::Primary, label).clicked() {
                            result = if self.purpose.saving() {
                                self.save_path().map(|p| vec![p])
                            } else {
                                Some(self.selected.clone())
                            };
                        }
                    });
                    if widgets::button(ui, look, ButtonKind::Normal, "Cancel").clicked() {
                        result = Some(Vec::new());
                    }
                    if self.purpose == Purpose::ImportModel {
                        ui.label(
                            RichText::new(
                                "Ctrl+click picks several. You can also drop files on the window.",
                            )
                            .small()
                            .color(look.pal.text_faint),
                        );
                    }
                    if let Some(p) = self
                        .save_path()
                        .filter(|p| self.purpose.saving() && p.exists())
                    {
                        ui.label(
                            RichText::new(format!(
                                "{} will be replaced",
                                p.file_name().unwrap_or_default().to_string_lossy()
                            ))
                            .small()
                            .color(look.pal.accent_text),
                        );
                    }
                });
            });
        if !open {
            result = Some(Vec::new());
        }
        result
    }

    /// The folder the dialog shows (remembered for the next one).
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}
