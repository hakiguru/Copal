//! Personal resins in the GUI: the "My resin" form in the Resin tab, importing preset files and
//! watching the resin folder, so a file dropped there shows up without a restart.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use copal_core::presets::Kind;
use copal_core::{library, script::quote};
use egui::{RichText, TextEdit, Ui};

use crate::files::Purpose;
use crate::look::Look;
use crate::widgets::{self, ButtonKind};
use crate::workspace::{LineKind, Workspace};

/// How often the resin folder is checked for new or changed files.
const WATCH_EVERY: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct ResinState {
    pub brand: String,
    pub name: String,
    pub color: String,
    /// The resin the form was last filled from, so switching resins refills it.
    filled_from: Option<String>,
    /// Ids of the resins that come from the personal folder.
    personal: Vec<String>,
    /// Files in the folder that could not be read.
    pub problems: Vec<String>,
    signature: Option<u64>,
    checked: Option<Instant>,
}

/// Opens a folder in the system file manager. Errors are returned for the console.
fn open_in_file_manager(dir: &Path) -> Result<(), String> {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not start {program}: {e}"))
}

impl Workspace {
    /// Reloads personal resins when a file in their folder was added, removed or changed.
    /// Call once per frame; it only looks at the disk every 1.5 s.
    pub fn poll_presets(&mut self, ctx: &egui::Context) {
        ctx.request_repaint_after(WATCH_EVERY);
        if self
            .resins
            .checked
            .is_some_and(|t| t.elapsed() < WATCH_EVERY)
        {
            return;
        }
        self.resins.checked = Some(Instant::now());
        let Some(user) = copal_core::user_presets() else {
            return;
        };
        let sig = user.signature(Kind::Resin);
        let first = self.resins.signature.is_none();
        if self.resins.signature == Some(sig) {
            return;
        }
        self.resins.signature = Some(sig);
        self.reload_resins(!first);
    }

    /// Reads the personal resin folder again. With `announce`, says so in the console.
    fn reload_resins(&mut self, announce: bool) {
        let problems = copal_core::reload_user_library();
        if let Some(user) = copal_core::user_presets() {
            self.resins.personal = user
                .resin_files()
                .into_iter()
                .filter_map(|(_, r)| r.ok().map(|r| r.id))
                .collect();
        }
        if announce {
            self.log(
                LineKind::Info,
                format!(
                    "Resin folder changed: {} personal resins",
                    self.resins.personal.len()
                ),
            );
        }
        for p in &problems {
            if !self.resins.problems.contains(p) {
                self.log(LineKind::Error, format!("resin preset skipped: {p}"));
            }
        }
        self.resins.problems = problems;
    }

    /// Copies resin preset files into the personal folder and picks the first one that has
    /// settings for the current printer.
    pub fn import_resins(&mut self, paths: &[PathBuf]) {
        let Some(user) = copal_core::user_presets() else {
            self.log(
                LineKind::Error,
                "this system has no config folder for presets",
            );
            return;
        };
        let mut pick = None;
        for p in paths {
            match user.import_resin(p) {
                Ok((r, dest)) => {
                    self.log(
                        LineKind::Info,
                        format!("Imported {} → {}", r.title(), dest.display()),
                    );
                    match &self.doc().printer {
                        Some(printer) if r.profile_for(printer).is_none() => {
                            let list: Vec<&str> =
                                r.profiles.iter().map(|p| p.printer.as_str()).collect();
                            self.log(
                                LineKind::Error,
                                format!(
                                    "{} has no settings for this printer, only for: {}",
                                    r.title(),
                                    list.join(", ")
                                ),
                            );
                        }
                        _ => {
                            pick.get_or_insert(r.id);
                        }
                    }
                }
                Err(e) => self.log(LineKind::Error, e.to_string()),
            }
        }
        self.reload_resins(false);
        self.resins.signature = Some(user.signature(Kind::Resin));
        if let Some(id) = pick
            && self.doc().printer.is_some()
        {
            self.run(&format!("project.set_resin resin={}", quote(&id)));
        }
    }

    /// Opens the personal resin folder in the file manager (and prints its path).
    pub fn open_resin_folder(&mut self) {
        let Some(user) = copal_core::user_presets() else {
            self.log(
                LineKind::Error,
                "this system has no config folder for presets",
            );
            return;
        };
        match user.ensure_folder(Kind::Resin) {
            Ok(dir) => {
                self.log(LineKind::Info, format!("Resin presets: {}", dir.display()));
                if let Err(e) = open_in_file_manager(&dir) {
                    self.log(LineKind::Error, e);
                }
            }
            Err(e) => self.log(LineKind::Error, e.to_string()),
        }
    }

    /// The "My resin" part of the Resin tab.
    pub(crate) fn my_resin_section(&mut self, ui: &mut Ui, look: &Look) {
        let p = look.pal;
        widgets::section(ui, look, "My resin");
        let current = self.doc().resin.clone();
        let current_resin = current
            .as_deref()
            .and_then(|id| library().resin(id).cloned());
        // Fill the form from the chosen resin, so "save" starts from its brand, name and color.
        if self.resins.filled_from != current {
            self.resins.filled_from = current.clone();
            if let Some(r) = &current_resin {
                let mine = current
                    .as_ref()
                    .is_some_and(|id| self.resins.personal.contains(id));
                self.resins.brand = if r.brand == "My" {
                    String::new()
                } else {
                    r.brand.clone()
                };
                // A built-in resin keeps its own name; yours gets a new one.
                self.resins.name = if mine { r.name.clone() } else { String::new() };
                self.resins.color = r.color.clone();
            }
        }
        let personal = current
            .as_ref()
            .is_some_and(|id| self.resins.personal.contains(id));
        ui.label(
            RichText::new(
                "Saves the settings above (exposure, waits, lift) as a resin for this printer. \
                 It shows up in the resin list and stays when Copal updates.",
            )
            .small()
            .color(p.text_faint),
        );
        let w = ui.available_width();
        egui::Grid::new("my-resin")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                for (label, text, hint) in [
                    ("Brand", &mut self.resins.brand, "My"),
                    ("Name", &mut self.resins.name, "ABS-Like 30 µm"),
                    ("Color", &mut self.resins.color, "Grey"),
                ] {
                    ui.label(RichText::new(label).color(p.text_weak));
                    ui.add(
                        TextEdit::singleline(text)
                            .hint_text(hint)
                            .desired_width((w - 70.0).max(80.0)),
                    );
                    ui.end_row();
                }
            });
        let name = self.resins.name.trim().to_string();
        let can_save = !name.is_empty() && self.doc().printer.is_some();
        let new_id = format!("my/{}", copal_core::presets::slug(&name));
        let overwrites = library().resin(&new_id).is_some();
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(can_save, |ui| {
                let label = if overwrites {
                    "Update my resin"
                } else {
                    "Save as my resin"
                };
                let hint = if overwrites {
                    "Replaces the settings of this personal resin for the current printer"
                } else {
                    "Adds a new resin with the current settings"
                };
                if widgets::button(ui, look, ButtonKind::Primary, label)
                    .on_hover_text(hint)
                    .clicked()
                {
                    let mut line = format!("presets save resin {}", quote(&name));
                    for (key, v) in [("brand", &self.resins.brand), ("color", &self.resins.color)] {
                        if !v.trim().is_empty() {
                            line.push_str(&format!(" {}", quote(&format!("{key}={}", v.trim()))));
                        }
                    }
                    self.run(&line);
                    self.run(&format!("project.set_resin resin={}", quote(&new_id)));
                    self.reload_resins(false);
                }
            });
            if personal
                && let Some(id) = current.clone()
                && widgets::button(ui, look, ButtonKind::Normal, "Delete")
                    .on_hover_text("Deletes the file of this personal resin")
                    .clicked()
            {
                self.run(&format!("presets delete resin {}", quote(&id)));
                self.reload_resins(false);
            }
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if widgets::button(ui, look, ButtonKind::Normal, "Import file…")
                .on_hover_text("Adds a resin preset (.toml), for example one a friend shared. You can also drop it on the window.")
                .clicked()
            {
                self.open_dialog(Purpose::ImportResin);
            }
            if widgets::button(ui, look, ButtonKind::Normal, "Open folder")
                .on_hover_text("Every .toml file in this folder is a resin. Files you put there show up in a few seconds.")
                .clicked()
            {
                self.open_resin_folder();
            }
        });
        if let Some(dir) = copal_core::user_presets().map(|u| u.folder(Kind::Resin)) {
            ui.label(
                RichText::new(format!("Folder: {}", dir.display()))
                    .small()
                    .color(p.text_faint),
            );
        }
        for problem in &self.resins.problems {
            ui.label(
                RichText::new(format!("Skipped {problem}"))
                    .small()
                    .color(p.error),
            );
        }
    }
}
