1. All work for the version is merged into `main`.
2. `version` in the root `Cargo.toml` is bumped.
3. `CHANGELOG.md` gets a `## X.Y.Z` section, in plain English.
4. When that reaches `main`, the release workflow creates the tag `vX.Y.Z`, builds every package for Copal, Copal Lite and Copal CLI and creates a **GitHub Release** with the changelog and all files.
5. Every release is marked as the latest one, so it shows up on the repository page.

Fix releases (`0.3.1`) follow the same steps.
