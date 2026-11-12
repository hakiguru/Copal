//! The name and icon of the app in the taskbar and the dock.
//!
//! Linux desktops take both from a `.desktop` file whose name matches the window's app id. The
//! packages (deb, rpm, Flatpak, AUR) install one. An AppImage or the `.tar.gz` does not, so the
//! dock shows a gear and the raw app id. For those, the app writes a `.desktop` file and its icon
//! into the user's data folder at start (like most AppImages do) and keeps the path to the
//! program in it up to date.

/// The name people see: "Copal Slicer 0.4.0" or "Copal Slicer Lite 0.4.0".
pub fn app_name(edition: crate::Edition) -> String {
    let version = env!("CARGO_PKG_VERSION");
    match edition {
        crate::Edition::Full => format!("Copal Slicer {version}"),
        crate::Edition::Lite => format!("Copal Slicer Lite {version}"),
    }
}

/// The app id of the window, the same as the name of the `.desktop` file.
pub fn app_id(edition: crate::Edition) -> &'static str {
    match edition {
        crate::Edition::Full => "io.github.hakiguru.Copal",
        crate::Edition::Lite => "io.github.hakiguru.Copal.Lite",
    }
}

/// Marks the files Copal wrote itself, so it only ever changes its own.
#[cfg(target_os = "linux")]
const MARK: &str = "X-Copal-Generated=true";

/// Makes sure the desktop knows the app's name and icon. Does nothing when a package already
/// installed them, or on other systems. Errors are ignored: this is only cosmetic.
pub fn integrate(edition: crate::Edition) {
    #[cfg(target_os = "linux")]
    {
        let _ = linux::integrate(edition);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = edition;
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::path::{Path, PathBuf};

    fn data_dirs() -> (Option<PathBuf>, Vec<PathBuf>) {
        let home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
        let system = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".into())
            .split(':')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect();
        (home, system)
    }

    /// The text of the `.desktop` file for this program.
    pub(super) fn entry(edition: crate::Edition, exec: &Path, icon: &Path) -> String {
        let (name, comment) = match edition {
            crate::Edition::Full => (
                "Copal Slicer",
                "Prepare 3D models for resin (MSLA) printers",
            ),
            crate::Edition::Lite => (
                "Copal Slicer Lite",
                "Copal in a simple, fast interface for weak computers",
            ),
        };
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={name}\n\
             GenericName=Resin 3D printing slicer\n\
             Comment={comment}\n\
             Exec=\"{exec}\" %F\n\
             Icon={icon}\n\
             Terminal=false\n\
             Categories=Graphics;3DGraphics;Engineering;\n\
             MimeType=application/x-copal-project;model/stl;model/obj;model/3mf;\n\
             StartupWMClass={id}\n\
             {MARK}\n",
            exec = exec.display(),
            icon = icon.display(),
            id = app_id(edition),
        )
    }

    pub(super) fn integrate(edition: crate::Edition) -> std::io::Result<()> {
        let id = app_id(edition);
        let file = format!("{id}.desktop");
        let (Some(home), system) = data_dirs() else {
            return Ok(());
        };
        // A package installed it: leave it alone.
        if system
            .iter()
            .any(|d| d.join("applications").join(&file).exists())
        {
            return Ok(());
        }
        let path = home.join("applications").join(&file);
        if let Ok(old) = std::fs::read_to_string(&path)
            && !old.contains(MARK)
        {
            // The user's own file.
            return Ok(());
        }
        // The AppImage itself, not the copy unpacked in /tmp.
        let exec = std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok())
            .ok_or_else(|| std::io::Error::other("no program path"))?;
        let icon_dir = home.join("icons/hicolor/256x256/apps");
        std::fs::create_dir_all(&icon_dir)?;
        let icon = icon_dir.join(format!("{id}.png"));
        if !icon.exists() {
            std::fs::write(&icon, crate::look::ICON_PNG)?;
        }
        let text = entry(edition, &exec, &icon);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            std::fs::create_dir_all(path.parent().unwrap_or(&home))?;
            std::fs::write(&path, text)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_carry_the_version() {
        assert!(app_name(crate::Edition::Full).starts_with("Copal Slicer 0."));
        assert!(app_name(crate::Edition::Lite).starts_with("Copal Slicer Lite 0."));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_entry_names_the_program_and_its_window() {
        let e = linux::entry(
            crate::Edition::Full,
            std::path::Path::new("/home/u/Copal.AppImage"),
            std::path::Path::new("/home/u/.local/share/icons/copal.png"),
        );
        assert!(e.contains("Name=Copal Slicer\n"));
        assert!(e.contains("Exec=\"/home/u/Copal.AppImage\" %F"));
        assert!(e.contains("StartupWMClass=io.github.hakiguru.Copal\n"));
        assert!(e.contains(MARK));
    }
}
