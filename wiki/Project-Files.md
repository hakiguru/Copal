> [!NOTE]
> 🚧 This page describes a planned feature. It arrives in **Copal 0.0**. Details may change.


A **`.copal`** file saves everything: models, positions, supports, settings, printer and resin.

- It opens in Copal, Copal Lite and Copal CLI.
- It is a zip archive inside: `project.json`, meshes, supports and a thumbnail.
- Newer Copal versions open older projects automatically.

**Autosave and backups** *(0.4)*: Copal saves a backup copy every few minutes in
`~/.local/share/copal/backups` (Linux) or `%LOCALAPPDATA%\Copal\backups` (Windows).
No cloud, no account.
