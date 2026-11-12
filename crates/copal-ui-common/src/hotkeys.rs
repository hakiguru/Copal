//! Keyboard shortcuts: every action with its default keys, the keys the user picked instead
//! (stored in the app settings, `key.<action>`), the window that shows and changes them (F1) and
//! the text for the `hotkeys` console command. What the actions do is in
//! [`Workspace::hotkeys`](crate::workspace::Workspace::hotkeys).

use std::collections::{BTreeMap, HashSet};

use copal_core::Settings;
use egui::{Event, Key, Modifiers, RichText, ScrollArea};

use crate::look::Look;
use crate::widgets;
use crate::workspace::Workspace;

/// One key with modifiers, like `Ctrl+Shift+Z`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binding {
    pub key: Key,
    /// Ctrl, or Cmd on a Mac.
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Binding {
    pub fn new(key: Key, m: Modifiers) -> Self {
        Self {
            key,
            ctrl: m.command || m.ctrl,
            shift: m.shift,
            alt: m.alt,
        }
    }

    /// Reads `Ctrl+Shift+Z`, `Alt+Up`, `F1`, `[` and similar. Case does not matter.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let (mut ctrl, mut shift, mut alt) = (false, false, false);
        let mut rest = text;
        while let Some((head, tail)) = rest.split_once('+') {
            // `Ctrl++` is Ctrl and the plus key.
            if tail.is_empty() {
                break;
            }
            match head.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" | "command" | "control" => ctrl = true,
                "shift" => shift = true,
                "alt" | "option" => alt = true,
                _ => return None,
            }
            rest = tail;
        }
        let name = rest.trim();
        let key = Key::from_name(name)
            .or_else(|| {
                let mut c = name.chars();
                let first = c.next()?.to_ascii_uppercase();
                Key::from_name(&format!("{first}{}", c.as_str().to_ascii_lowercase()))
            })
            .or_else(|| {
                Key::ALL
                    .iter()
                    .copied()
                    .find(|k| k.name().eq_ignore_ascii_case(name))
            })?;
        Some(Self {
            key,
            ctrl,
            shift,
            alt,
        })
    }

    /// Parses a list: `Ctrl+Shift+Z, Ctrl+Y`. Items are separated by a comma and a space, so
    /// `Ctrl+,` (Ctrl and comma) still works.
    pub fn parse_list(text: &str) -> Option<Vec<Self>> {
        if text.trim().is_empty() {
            return Some(Vec::new());
        }
        text.split(", ").map(Self::parse).collect()
    }

    fn modifier_count(&self) -> u8 {
        self.ctrl as u8 + self.shift as u8 + self.alt as u8
    }

    fn matches(&self, key: Key, m: Modifiers) -> bool {
        self.key == key
            && self.ctrl == (m.command || m.ctrl)
            && self.shift == m.shift
            && self.alt == m.alt
    }
}

impl std::fmt::Display for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        f.write_str(match self.key {
            Key::Comma => ",",
            Key::Period => ".",
            Key::OpenBracket => "[",
            Key::CloseBracket => "]",
            Key::Equals => "=",
            Key::Minus => "-",
            Key::Backtick => "`",
            Key::Semicolon => ";",
            Key::Slash => "/",
            Key::Backslash => "\\",
            Key::Quote => "'",
            Key::Escape => "Esc",
            k => k.name(),
        })
    }
}

/// Writes a list of keys the way [`Binding::parse_list`] reads it.
pub fn format_list(keys: &[Binding]) -> String {
    keys.iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// An action a key can do.
pub struct Action {
    /// Name in the settings: `key.<id>`.
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    /// Default keys, as [`Binding::parse_list`] reads them.
    pub keys: &'static str,
}

const fn a(
    id: &'static str,
    group: &'static str,
    label: &'static str,
    keys: &'static str,
) -> Action {
    Action {
        id,
        group,
        label,
        keys,
    }
}

const PROJECT: &str = "Project";
const STEPS: &str = "Steps and windows";
const VIEW: &str = "3D view";
const LAYOUT: &str = "Layout";
const PREPARE: &str = "Prepare";
const EXPORT: &str = "Export";

/// Every action with its default keys, in the order the window shows them.
pub const ACTIONS: &[Action] = &[
    a("open", PROJECT, "Open a project", "Ctrl+O"),
    a("save", PROJECT, "Save", "Ctrl+S"),
    a("save_as", PROJECT, "Save as", "Ctrl+Shift+S"),
    a("import", PROJECT, "Import models", "Ctrl+I"),
    a("export", PROJECT, "Slice and export", "Ctrl+E"),
    a("undo", PROJECT, "Undo", "Ctrl+Z"),
    a("redo", PROJECT, "Redo", "Ctrl+Shift+Z, Ctrl+Y"),
    a("preferences", PROJECT, "Preferences", "Ctrl+,"),
    a("history", PROJECT, "Print history", "Ctrl+Shift+P"),
    a("shortcuts", PROJECT, "Keyboard shortcuts (this list)", "F1"),
    a("step_layout", STEPS, "Layout step", "1"),
    a("step_prepare", STEPS, "Prepare step", "2"),
    a("step_export", STEPS, "Export step", "3"),
    a("panel_tools", STEPS, "Show or hide tools", "Alt+1"),
    a("panel_settings", STEPS, "Show or hide settings", "Alt+2"),
    a(
        "panel_console",
        STEPS,
        "Show or hide the console",
        "Alt+3, Alt+F12",
    ),
    a("console", STEPS, "Type in the console", "`"),
    a(
        "cancel",
        STEPS,
        "Cancel an export, leave a tool, deselect",
        "Esc",
    ),
    a("view_iso", VIEW, "Iso view", "4"),
    a("view_top", VIEW, "Top view", "5"),
    a("view_front", VIEW, "Front view", "6"),
    a("view_side", VIEW, "Side view", "7"),
    a(
        "view_fit",
        VIEW,
        "Show the whole plate and every model",
        "F",
    ),
    a("overhangs", VIEW, "Overhang colors on or off", "O"),
    a("select_next", VIEW, "Select the next model", "N"),
    a(
        "select_previous",
        VIEW,
        "Select the previous model",
        "Shift+N",
    ),
    a("layer_up", VIEW, "Layer slider: one layer up", "Alt+Up"),
    a(
        "layer_down",
        VIEW,
        "Layer slider: one layer down",
        "Alt+Down",
    ),
    a(
        "layer_up_10",
        VIEW,
        "Layer slider: ten layers up",
        "Alt+Shift+Up",
    ),
    a(
        "layer_down_10",
        VIEW,
        "Layer slider: ten layers down",
        "Alt+Shift+Down",
    ),
    a(
        "layer_all",
        VIEW,
        "Layer slider: show every layer",
        "Alt+Home",
    ),
    a("mode_move", LAYOUT, "Move mode", "G"),
    a("mode_rotate", LAYOUT, "Rotate mode", "R"),
    a("mode_scale", LAYOUT, "Scale mode", "S"),
    a("move_right", LAYOUT, "Move right by one snap step", "Right"),
    a("move_left", LAYOUT, "Move left by one snap step", "Left"),
    a("move_back", LAYOUT, "Move back by one snap step", "Up"),
    a(
        "move_front",
        LAYOUT,
        "Move to the front by one snap step",
        "Down",
    ),
    a(
        "move_right_10",
        LAYOUT,
        "Move right by ten steps",
        "Shift+Right",
    ),
    a(
        "move_left_10",
        LAYOUT,
        "Move left by ten steps",
        "Shift+Left",
    ),
    a("move_back_10", LAYOUT, "Move back by ten steps", "Shift+Up"),
    a(
        "move_front_10",
        LAYOUT,
        "Move to the front by ten steps",
        "Shift+Down",
    ),
    a("lift", LAYOUT, "Lift by one snap step", "PageUp"),
    a("lower", LAYOUT, "Lower by one snap step", "PageDown"),
    a("turn_left", LAYOUT, "Turn left by one snap step", "["),
    a("turn_right", LAYOUT, "Turn right by one snap step", "]"),
    a("bigger", LAYOUT, "Bigger by one snap step", "=, Plus"),
    a("smaller", LAYOUT, "Smaller by one snap step", "-"),
    a("on_plate", LAYOUT, "Put on the plate", "P"),
    a("center", LAYOUT, "Center on the plate", "C"),
    a("mirror_x", LAYOUT, "Mirror X", "Alt+X"),
    a("mirror_y", LAYOUT, "Mirror Y", "Alt+Y"),
    a("mirror_z", LAYOUT, "Mirror Z", "Alt+Z"),
    a("reset_rotation", LAYOUT, "Reset rotation", "Ctrl+R"),
    a(
        "orient",
        LAYOUT,
        "Auto orient: the best way up for printing",
        "Ctrl+T",
    ),
    a("duplicate", LAYOUT, "Duplicate", "Ctrl+D"),
    a(
        "delete",
        LAYOUT,
        "Delete the model or the selected supports",
        "X, Delete",
    ),
    a("arrange", LAYOUT, "Arrange all models", "Shift+A"),
    a(
        "prepare",
        PREPARE,
        "Prepare for printing: orient, hollow, holes, supports",
        "Ctrl+P",
    ),
    a("tool_view", PREPARE, "Click tool: view", "V"),
    a("tool_select", PREPARE, "Click tool: select supports", "B"),
    a("tool_support", PREPARE, "Click tool: add a support", "A"),
    a("tool_hole", PREPARE, "Click tool: add a drain hole", "H"),
    a("auto_supports", PREPARE, "Auto supports", "Ctrl+U"),
    a(
        "remove_supports",
        PREPARE,
        "Remove supports",
        "Ctrl+Shift+U",
    ),
    a("supports_light", PREPARE, "Light supports", "Ctrl+1"),
    a("supports_medium", PREPARE, "Medium supports", "Ctrl+2"),
    a("supports_heavy", PREPARE, "Heavy supports", "Ctrl+3"),
    a("islands", PREPARE, "Check islands", "I"),
    a("hollow", PREPARE, "Hollow", "Ctrl+H"),
    a("unhollow", PREPARE, "Make solid again", "Ctrl+Shift+H"),
    a(
        "holes",
        PREPARE,
        "Drain holes at the lowest points",
        "Ctrl+J",
    ),
    a("layer_preview", EXPORT, "Layer preview on or off", "L"),
];

/// Mouse and fixed keys: shown in the list, not changed.
pub const FIXED: &[(&str, &[(&str, &str)])] = &[
    (
        "Mouse",
        &[
            ("Left drag", "Turn the view, or move a model over the plate"),
            ("Right or middle drag, Shift+drag", "Pan"),
            ("Wheel", "Zoom to the cursor"),
            ("Double-click", "Show the whole plate and every model"),
            ("Ctrl while dragging", "Snap to steps"),
            ("Shift+click", "Add a support to the selection"),
        ],
    ),
    (
        "Layer preview and console",
        &[
            (
                "Up / Down, wheel",
                "Next / previous layer in the layer preview",
            ),
            ("Up / Down", "Earlier / later commands in the console"),
            ("Tab", "Complete a command, setting or name in the console"),
        ],
    ),
];

pub fn action(id: &str) -> Option<&'static Action> {
    ACTIONS.iter().find(|a| a.id == id)
}

/// The keys of every action: defaults with the user's changes on top.
#[derive(Debug, Clone, Default)]
pub struct Keymap {
    keys: BTreeMap<&'static str, Vec<Binding>>,
}

impl Keymap {
    pub fn defaults() -> Self {
        Self::from_settings(&Settings::default())
    }

    /// Keys from the settings. A key that cannot be read keeps the default.
    pub fn from_settings(s: &Settings) -> Self {
        let keys = ACTIONS
            .iter()
            .map(|a| {
                let user = s.keys.get(a.id).and_then(|t| Binding::parse_list(t));
                let keys = user.unwrap_or_else(|| Binding::parse_list(a.keys).unwrap_or_default());
                (a.id, keys)
            })
            .collect();
        Self { keys }
    }

    pub fn keys(&self, id: &str) -> &[Binding] {
        self.keys.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The keys of an action as text, e.g. `Ctrl+T`; "—" when it has none.
    pub fn text(&self, id: &str) -> String {
        let k = self.keys(id);
        if k.is_empty() {
            "—".into()
        } else {
            format_list(k)
        }
    }

    /// Actions other than `id` that use `key`.
    pub fn conflicts(&self, id: &str, key: Binding) -> Vec<&'static str> {
        self.keys
            .iter()
            .filter(|(other, keys)| **other != id && keys.contains(&key))
            .map(|(other, _)| *other)
            .collect()
    }

    /// The actions whose keys were pressed this frame. Keys with Ctrl or Alt are taken out of
    /// the input, so text fields do not see them too. While the user types in a field, only
    /// keys with Ctrl work.
    pub fn pressed(&self, ctx: &egui::Context, typing: bool) -> HashSet<&'static str> {
        let mut hit = HashSet::new();
        // Longer combinations first, so Ctrl+Shift+Z is not also read as Ctrl+Z.
        let mut all: Vec<(&'static str, Binding)> = self
            .keys
            .iter()
            .flat_map(|(id, keys)| keys.iter().map(move |b| (*id, *b)))
            .collect();
        all.sort_by_key(|(_, b)| std::cmp::Reverse(b.modifier_count()));
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                let Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } = e
                else {
                    return true;
                };
                let found = all.iter().find(|(_, b)| {
                    (b.matches(*key, *modifiers)
                        || physical_key.is_some_and(|p| p != *key && b.matches(p, *modifiers)))
                        && (!typing || b.ctrl || is_function_key(b.key))
                });
                match found {
                    Some((id, b)) => {
                        hit.insert(*id);
                        // Plain keys stay: the 3D view and the layer preview read arrows too.
                        !(b.ctrl || b.alt)
                    }
                    None => true,
                }
            })
        });
        hit
    }
}

/// Shift, Ctrl, Alt and Super pressed alone: they only change other keys.
fn is_modifier(k: Key) -> bool {
    matches!(
        k,
        Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight
            | Key::SuperLeft
            | Key::SuperRight
    )
}

/// F1 … F20 work while typing too: they never type anything.
fn is_function_key(k: Key) -> bool {
    k.name().len() >= 2
        && k.name().starts_with('F')
        && k.name()[1..].chars().all(|c| c.is_ascii_digit())
}

/// The table as plain lines, for the console.
pub fn hotkey_lines(settings: &Settings) -> Vec<String> {
    let map = Keymap::from_settings(settings);
    let mut out = Vec::new();
    let mut group = "";
    for a in ACTIONS {
        if a.group != group {
            group = a.group;
            out.push(format!("{group}:"));
        }
        out.push(format!(
            "  {:<28} {:<44} key.{}",
            map.text(a.id),
            a.label,
            a.id
        ));
    }
    for (group, keys) in FIXED {
        out.push(format!("{group}:"));
        for (key, what) in *keys {
            out.push(format!("  {key:<28} {what}"));
        }
    }
    out.push(
        "Change keys in the shortcut window (F1) or with `copal-cli settings key.<action>=Ctrl+K`."
            .into(),
    );
    out
}

impl Workspace {
    /// The window with every shortcut, where keys can be changed. Open with F1 or `Keys`.
    pub fn hotkeys_window(&mut self, ctx: &egui::Context, look: &Look) {
        if !self.show_hotkeys {
            self.rebinding = None;
            return;
        }
        let map = Keymap::from_settings(&self.settings);
        let mut open = true;
        let mut change: Option<(&'static str, Option<String>)> = None;
        let mut reset_all = false;
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .collapsible(false)
            .default_size([600.0, 600.0])
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "Click a key to change it, then press the new keys. Esc cancels, \
                         Backspace removes the key. Letter keys work when you are not typing.",
                    )
                    .small()
                    .color(look.pal.text_faint),
                );
                if !self.settings.keys.is_empty()
                    && widgets::button(ui, look, widgets::ButtonKind::Normal, "Reset all keys")
                        .clicked()
                {
                    reset_all = true;
                }
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut group = "";
                        for a in ACTIONS {
                            if a.group != group {
                                group = a.group;
                                widgets::section(ui, look, group);
                            }
                            ui.horizontal(|ui| {
                                let waiting = self.rebinding == Some(a.id);
                                let text = if waiting {
                                    "press keys…".to_string()
                                } else {
                                    map.text(a.id)
                                };
                                let b = ui.add_sized(
                                    [150.0, 20.0],
                                    egui::Button::new(RichText::new(text).monospace().color(
                                        if waiting {
                                            look.pal.accent
                                        } else {
                                            look.pal.accent_text
                                        },
                                    )),
                                );
                                if b.on_hover_text("Click, then press the new keys").clicked() {
                                    self.rebinding = Some(a.id);
                                }
                                ui.label(RichText::new(a.label).color(look.pal.text));
                                let changed = self.settings.keys.contains_key(a.id);
                                if changed
                                    && ui
                                        .small_button("default")
                                        .on_hover_text(format!("Back to {}", a.keys))
                                        .clicked()
                                {
                                    change = Some((a.id, None));
                                }
                                // Same keys on another action: say so.
                                for k in map.keys(a.id) {
                                    let other = map.conflicts(a.id, *k);
                                    if let Some(o) = other.first().and_then(|o| action(o)) {
                                        ui.label(
                                            RichText::new(format!("also: {}", o.label))
                                                .small()
                                                .color(look.pal.error),
                                        );
                                    }
                                }
                            });
                        }
                        for (group, keys) in FIXED {
                            widgets::section(ui, look, group);
                            egui::Grid::new(("fixed-keys", *group))
                                .num_columns(2)
                                .spacing([18.0, 5.0])
                                .show(ui, |ui| {
                                    for (key, what) in *keys {
                                        ui.label(
                                            RichText::new(*key)
                                                .monospace()
                                                .color(look.pal.text_weak),
                                        );
                                        ui.label(RichText::new(*what).color(look.pal.text));
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            });
        let mut s = self.settings.clone();
        if reset_all {
            s.keys.clear();
        }
        if let Some((id, keys)) = change {
            match keys {
                Some(k) => s.keys.insert(id.to_string(), k),
                None => s.keys.remove(id),
            };
        }
        if s != self.settings {
            self.set_settings(s);
        }
        self.show_hotkeys = open;
    }

    /// While a key is being changed: takes the next key press for it. Returns true when the
    /// keyboard was used for that this frame.
    pub(crate) fn capture_rebinding(&mut self, ctx: &egui::Context) -> bool {
        let Some(id) = self.rebinding else {
            return false;
        };
        let mut got: Option<Option<Binding>> = None;
        ctx.input_mut(|i| {
            i.events.retain(|e| match e {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if !is_modifier(*key) => {
                    if got.is_none() {
                        got = Some(match key {
                            Key::Escape => None,
                            Key::Backspace => Some(Binding {
                                key: Key::Backspace,
                                ctrl: true,
                                shift: true,
                                alt: true,
                            }),
                            k => Some(Binding::new(*k, *modifiers)),
                        });
                    }
                    false
                }
                Event::Text(_) => false,
                _ => true,
            })
        });
        match got {
            None => {}
            Some(None) => self.rebinding = None,
            Some(Some(b)) => {
                let mut s = self.settings.clone();
                // Backspace (marked above with every modifier) removes the keys.
                let text = if b.key == Key::Backspace && b.ctrl && b.shift && b.alt {
                    String::new()
                } else {
                    b.to_string()
                };
                let default = action(id).map(|a| a.keys).unwrap_or("");
                if text == default {
                    s.keys.remove(id);
                } else {
                    s.keys.insert(id.to_string(), text);
                }
                self.set_settings(s);
                self.rebinding = None;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_key_reads_and_no_action_is_listed_twice() {
        let mut seen = HashSet::new();
        for a in ACTIONS {
            assert!(seen.insert(a.id), "{} twice", a.id);
            let keys =
                Binding::parse_list(a.keys).unwrap_or_else(|| panic!("{}: {}", a.id, a.keys));
            assert!(!keys.is_empty(), "{}", a.id);
            // Written back the same way.
            let back = Binding::parse_list(&format_list(&keys)).unwrap();
            assert_eq!(keys, back, "{}", a.id);
        }
        let lines = hotkey_lines(&Settings::default());
        assert!(lines.iter().any(|l| l.contains("Ctrl+U")));
    }

    #[test]
    fn default_keys_do_not_clash() {
        let map = Keymap::defaults();
        for a in ACTIONS {
            for k in map.keys(a.id) {
                assert!(
                    map.conflicts(a.id, *k).is_empty(),
                    "{} and {:?}",
                    a.id,
                    map.conflicts(a.id, *k)
                );
            }
        }
    }

    #[test]
    fn keys_parse_and_user_keys_win() {
        let b = Binding::parse("ctrl+shift+z").unwrap();
        assert!(b.ctrl && b.shift && !b.alt && b.key == Key::Z);
        assert_eq!(Binding::parse("Ctrl+,").unwrap().key, Key::Comma);
        assert_eq!(Binding::parse("Alt+F12").unwrap().key, Key::F12);
        assert_eq!(Binding::parse("pageup").unwrap().key, Key::PageUp);
        assert!(Binding::parse("Hyper+Q").is_none());
        let list = Binding::parse_list("Ctrl+,, Ctrl+Y").unwrap();
        assert_eq!(list.len(), 2);
        let mut s = Settings::default();
        s.keys.insert("orient".into(), "Ctrl+Shift+O".into());
        s.keys.insert("arrange".into(), String::new());
        s.keys.insert("undo".into(), "nonsense+key".into());
        let map = Keymap::from_settings(&s);
        assert_eq!(map.text("orient"), "Ctrl+Shift+O");
        assert_eq!(map.text("arrange"), "—");
        // Keys that cannot be read keep the default.
        assert_eq!(map.text("undo"), "Ctrl+Z");
    }
}
