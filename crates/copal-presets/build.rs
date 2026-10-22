//! Embeds every file in `presets/` into the binary, so built-in presets need no install step.

use std::fmt::Write;
use std::path::Path;

fn walk(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|e| e == "toml") {
            out.push(p.canonicalize().unwrap().display().to_string());
        }
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut code = String::from("pub static BUILTIN: &[(&str, &str)] = &[\n");
    for f in &files {
        let rel = Path::new(f)
            .strip_prefix(root.canonicalize().unwrap())
            .unwrap();
        let rel = rel.display().to_string().replace('\\', "/");
        writeln!(code, "    ({rel:?}, include_str!({f:?})),").unwrap();
        println!("cargo:rerun-if-changed={f}");
    }
    code.push_str("];\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("builtin.rs");
    std::fs::write(out, code).unwrap();
}
