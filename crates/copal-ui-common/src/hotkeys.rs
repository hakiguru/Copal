//! Every keyboard shortcut of the apps in one table, the window that shows it (F1) and the text
//! for the `hotkeys` console command. The keys themselves are handled in
//! [`Workspace::hotkeys`](crate::workspace::Workspace::hotkeys) and in the 3D view.

use egui::{RichText, ScrollArea};

use crate::look::Look;
use crate::widgets;
use crate::workspace::Workspace;

/// Groups of (keys, what they do).
pub const HOTKEYS: &[(&str, &[(&str, &str)])] = &[
    (
        "Project",
        &[
            ("Ctrl+O", "Open a project"),
            ("Ctrl+S", "Save"),
            ("Ctrl+Shift+S", "Save as"),
            ("Ctrl+I", "Import models"),
            ("Ctrl+E", "Slice and export"),
            ("Ctrl+Z", "Undo"),
            ("Ctrl+Shift+Z, Ctrl+Y", "Redo"),
            ("Ctrl+,", "Preferences"),
            ("F1", "This list"),
        ],
    ),
    (
        "Steps and windows",
        &[
            ("1 / 2 / 3", "Layout / Prepare / Export"),
            (
                "Alt+1 / Alt+2 / Alt+3",
                "Show or hide tools / settings / console",
            ),
            ("`", "Type in the console"),
            ("Esc", "Cancel an export, leave a tool, deselect"),
        ],
    ),
    (
        "3D view",
        &[
            ("4 / 5 / 6 / 7", "Iso / top / front / side view"),
            ("F, double-click", "Show the whole plate and every model"),
            ("O", "Overhang colors on or off"),
            ("N / Shift+N", "Select the next / previous model"),
            ("Left drag", "Turn the view, or move a model over the plate"),
            ("Right or middle drag, Shift+drag", "Pan"),
            ("Wheel", "Zoom to the cursor"),
        ],
    ),
    (
        "Layout",
        &[
            ("G / R / S", "Move / rotate / scale handles"),
            ("Ctrl while dragging", "Snap to steps"),
            ("Arrows", "Move by one snap step (Shift: ten steps)"),
            ("Page Up / Page Down", "Lift / lower by one snap step"),
            ("[ / ]", "Turn left / right by one snap step"),
            ("= / -", "Bigger / smaller by one snap step"),
            ("P", "Put on the plate"),
            ("C", "Center on the plate"),
            ("Alt+X / Alt+Y / Alt+Z", "Mirror"),
            ("Ctrl+R", "Reset rotation"),
            ("Ctrl+T", "Auto orient: turn the best way up for printing"),
            ("Ctrl+D", "Duplicate"),
            ("X, Delete", "Delete the model"),
            ("Shift+A", "Arrange all models"),
        ],
    ),
    (
        "Prepare",
        &[
            (
                "Ctrl+P",
                "Prepare for printing: orient, hollow, holes, supports",
            ),
            (
                "V / B / A / H",
                "View / select supports / add support / add hole",
            ),
            ("Shift+click", "Add a support to the selection"),
            ("Delete", "Delete the selected supports"),
            ("Ctrl+U", "Auto supports"),
            ("Ctrl+Shift+U", "Remove supports"),
            (
                "Ctrl+1 / Ctrl+2 / Ctrl+3",
                "Light / medium / heavy supports",
            ),
            ("I", "Check islands"),
            ("Ctrl+H", "Hollow"),
            ("Ctrl+Shift+H", "Make solid again"),
            ("Ctrl+J", "Drain holes at the lowest points"),
        ],
    ),
    (
        "Export",
        &[
            ("L", "Layer preview on or off"),
            ("Up / Down, wheel", "Next / previous layer"),
        ],
    ),
    (
        "Console",
        &[
            ("Up / Down", "Earlier / later commands"),
            ("Tab", "Complete a command, setting or name"),
        ],
    ),
];

/// The table as plain lines, for the console.
pub fn hotkey_lines() -> Vec<String> {
    let mut out = Vec::new();
    for (group, keys) in HOTKEYS {
        out.push(format!("{group}:"));
        for (key, action) in *keys {
            out.push(format!("  {key:<34} {action}"));
        }
    }
    out
}

impl Workspace {
    /// The window with every shortcut. Open with F1 or the `hotkeys` button.
    pub fn hotkeys_window(&mut self, ctx: &egui::Context, look: &Look) {
        if !self.show_hotkeys {
            return;
        }
        let mut open = true;
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .collapsible(false)
            .default_size([520.0, 560.0])
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "Letter keys work when you are not typing in a field or the console.",
                    )
                    .small()
                    .color(look.pal.text_faint),
                );
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (group, keys) in HOTKEYS {
                            widgets::section(ui, look, group);
                            egui::Grid::new(("hotkeys", *group))
                                .num_columns(2)
                                .spacing([18.0, 5.0])
                                .show(ui, |ui| {
                                    for (key, action) in *keys {
                                        ui.label(
                                            RichText::new(*key)
                                                .monospace()
                                                .color(look.pal.accent_text),
                                        );
                                        ui.label(RichText::new(*action).color(look.pal.text));
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            });
        self.show_hotkeys = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_has_keys_and_no_key_is_listed_twice() {
        let mut seen = std::collections::HashSet::new();
        for (group, keys) in HOTKEYS {
            assert!(!keys.is_empty(), "{group}");
            for (key, action) in *keys {
                assert!(!key.is_empty() && !action.is_empty());
                assert!(seen.insert((*group, *key)), "{group}: {key} twice");
            }
        }
        let lines = hotkey_lines();
        assert!(lines.iter().any(|l| l.contains("Ctrl+U")));
        assert_eq!(
            lines.len(),
            HOTKEYS.len() + HOTKEYS.iter().map(|(_, k)| k.len()).sum::<usize>()
        );
    }
}
