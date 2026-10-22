//! The command system: every change to a [`Document`] is a [`Command`].
//!
//! The GUI, the CLI, the console and scripts all build commands; [`History`] applies them
//! and keeps undo/redo.

use std::fmt::Debug;

use thiserror::Error;

use crate::document::{Document, ObjectId};

#[derive(Debug, Error, PartialEq)]
pub enum CommandError {
    #[error("unknown command `{0}` (run `commands` to see the list)")]
    UnknownCommand(String),
    #[error("`{command}`: missing argument `{arg}`")]
    MissingArg { command: String, arg: String },
    #[error("`{command}`: unknown argument `{arg}`")]
    UnknownArg { command: String, arg: String },
    #[error("`{command}`: argument `{arg}` has a bad value `{value}`: {reason}")]
    BadValue {
        command: String,
        arg: String,
        value: String,
        reason: String,
    },
    #[error("syntax error: {0}")]
    Syntax(String),
    #[error("there is no object with id {0}")]
    NoSuchObject(ObjectId),
    #[error("{0}")]
    Invalid(String),
}

/// A change to the document.
///
/// `apply` performs the change and returns the command that undoes it.
pub trait Command: Debug + Send + Sync {
    /// Stable name, e.g. `object.move`.
    fn name(&self) -> &'static str;

    /// The command in the text language, if it can be written as text.
    /// Used for the console echo and macro recording.
    fn to_text(&self) -> Option<String>;

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError>;
}

/// Several commands applied as one step: one undo reverts them all.
#[derive(Debug)]
pub struct Batch {
    /// Shown in the history and echoed in the console.
    pub label: Option<String>,
    pub commands: Vec<Box<dyn Command>>,
}

impl Batch {
    pub fn new(label: impl Into<String>, commands: Vec<Box<dyn Command>>) -> Self {
        Self {
            label: Some(label.into()),
            commands,
        }
    }

    pub(crate) fn inverse(commands: Vec<Box<dyn Command>>) -> Self {
        Self {
            label: None,
            commands,
        }
    }
}

impl Command for Batch {
    fn name(&self) -> &'static str {
        "batch"
    }

    fn to_text(&self) -> Option<String> {
        self.label.clone()
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let mut inverses: Vec<Box<dyn Command>> = Vec::with_capacity(self.commands.len());
        for cmd in &self.commands {
            match cmd.apply(doc) {
                Ok(inv) => inverses.push(inv),
                Err(e) => {
                    // Roll back what was already applied, so a failed batch changes nothing.
                    for inv in inverses.into_iter().rev() {
                        let _ = inv.apply(doc);
                    }
                    return Err(e);
                }
            }
        }
        inverses.reverse();
        Ok(Box::new(Batch::inverse(inverses)))
    }
}

#[derive(Debug)]
struct Entry {
    /// What the user did (shown in the history panel).
    label: String,
    /// Applying this reverts the entry.
    inverse: Box<dyn Command>,
}

/// Undo/redo history for one document.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    /// Bumped on every change; lets UIs know when to redraw or mark the file dirty.
    revision: u64,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies `cmd` to `doc` and records it. Clears the redo stack.
    pub fn execute(
        &mut self,
        doc: &mut Document,
        cmd: Box<dyn Command>,
    ) -> Result<(), CommandError> {
        let label = cmd.to_text().unwrap_or_else(|| cmd.name().to_string());
        let inverse = cmd.apply(doc)?;
        self.undo.push(Entry { label, inverse });
        self.redo.clear();
        self.revision += 1;
        Ok(())
    }

    /// Reverts the last command. Returns its label, or `None` if there is nothing to undo.
    pub fn undo(&mut self, doc: &mut Document) -> Result<Option<String>, CommandError> {
        let Some(entry) = self.undo.pop() else {
            return Ok(None);
        };
        let redo = match entry.inverse.apply(doc) {
            Ok(redo) => redo,
            Err(e) => {
                self.undo.push(entry);
                return Err(e);
            }
        };
        let label = entry.label.clone();
        self.redo.push(Entry {
            label: entry.label,
            inverse: redo,
        });
        self.revision += 1;
        Ok(Some(label))
    }

    /// Re-applies the last undone command. Returns its label.
    pub fn redo(&mut self, doc: &mut Document) -> Result<Option<String>, CommandError> {
        let Some(entry) = self.redo.pop() else {
            return Ok(None);
        };
        let undo = match entry.inverse.apply(doc) {
            Ok(undo) => undo,
            Err(e) => {
                self.redo.push(entry);
                return Err(e);
            }
        };
        let label = entry.label.clone();
        self.undo.push(Entry {
            label: entry.label,
            inverse: undo,
        });
        self.revision += 1;
        Ok(Some(label))
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Labels of undoable steps, oldest first.
    pub fn undo_labels(&self) -> impl Iterator<Item = &str> {
        self.undo.iter().map(|e| e.label.as_str())
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}
