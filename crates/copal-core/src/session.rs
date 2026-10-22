//! A [`Session`] is what an app has open: the document, its history and its file.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::command::{Command, CommandError, History};
use crate::commands::parse_command;
use crate::document::Document;
use crate::project::{self, ProjectError};

#[derive(Debug, Error)]
pub enum RunError {
    #[error("line {line}: {source}")]
    Command { line: usize, source: CommandError },
}

#[derive(Debug, Default)]
pub struct Session {
    pub doc: Document,
    pub history: History,
    path: Option<PathBuf>,
    saved_revision: u64,
}

impl Session {
    pub fn new(doc: Document) -> Self {
        Self {
            doc,
            ..Default::default()
        }
    }

    pub fn open(path: &Path) -> Result<Self, ProjectError> {
        let doc = project::load(path)?;
        Ok(Self {
            doc,
            path: Some(path.to_path_buf()),
            ..Default::default()
        })
    }

    /// Opens a project as a new, unsaved copy (used for print history snapshots, so the
    /// snapshot itself is never overwritten).
    pub fn open_copy(path: &Path) -> Result<Self, ProjectError> {
        let doc = project::load(path)?;
        let mut s = Self::new(doc);
        // Unsaved until the user picks a place for it.
        s.saved_revision = u64::MAX;
        Ok(s)
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// True when there are changes that are not saved yet.
    pub fn is_dirty(&self) -> bool {
        self.history.revision() != self.saved_revision
    }

    pub fn save_as(&mut self, path: &Path) -> Result<(), ProjectError> {
        project::save(&self.doc, path)?;
        self.path = Some(path.to_path_buf());
        self.saved_revision = self.history.revision();
        Ok(())
    }

    /// Saves to the current path. Returns `Ok(false)` if the session has no path yet.
    pub fn save(&mut self) -> Result<bool, ProjectError> {
        match self.path.clone() {
            Some(p) => self.save_as(&p).map(|()| true),
            None => Ok(false),
        }
    }

    pub fn execute(&mut self, cmd: Box<dyn Command>) -> Result<(), CommandError> {
        self.history.execute(&mut self.doc, cmd)
    }

    /// Runs one line of the text language. Blank lines and comments do nothing.
    pub fn run_line(&mut self, line: &str) -> Result<(), CommandError> {
        match parse_command(line)? {
            Some(cmd) => self.execute(cmd),
            None => Ok(()),
        }
    }

    /// Runs a whole script. Stops at the first error and reports its line number.
    /// Lines before the error stay applied (and can be undone).
    pub fn run_script(&mut self, script: &str) -> Result<usize, RunError> {
        let mut count = 0;
        for (i, line) in script.lines().enumerate() {
            let before = self.history.revision();
            self.run_line(line).map_err(|source| RunError::Command {
                line: i + 1,
                source,
            })?;
            if self.history.revision() != before {
                count += 1;
            }
        }
        Ok(count)
    }
}
