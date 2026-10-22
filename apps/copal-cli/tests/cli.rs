use std::path::Path;
use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_copal-cli"))
        .args(args)
        .output()
        .unwrap()
}

fn ok(args: &[&str]) -> String {
    let out = cli(args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn full_terminal_workflow() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("part.copal");
    let f = file.to_str().unwrap();

    ok(&["new", f, "--printer", "elegoo/saturn-4-ultra-16k"]);
    ok(&["run", f, "object.add_cube", "size=10", "name=Test cube"]);
    ok(&["run", f, "object.move", "id=1", "x=-5"]);

    let script = dir.path().join("steps.copalscript");
    std::fs::write(
        &script,
        "# rotate and drop\nobject.rotate id=1 x=45\nobject.drop id=1\n",
    )
    .unwrap();
    assert!(ok(&["script", f, script.to_str().unwrap()]).contains("Applied 2 commands"));

    let info: serde_json::Value = serde_json::from_str(&ok(&["--json", "info", f])).unwrap();
    assert_eq!(info["name"], "part");
    assert_eq!(info["printer"], "elegoo/saturn-4-ultra-16k");
    assert_eq!(info["objects"][0]["name"], "Test cube");
    assert_eq!(info["objects"][0]["position"][0], -5.0);
    assert_eq!(info["objects"][0]["rotation_deg"][0], 45.0);
}

#[test]
fn dry_run_does_not_save() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("p.copal");
    let f = f.to_str().unwrap();
    ok(&["new", f]);
    ok(&["run", "--dry-run", f, "object.add_cube", "size=1"]);
    assert!(ok(&["info", f]).contains("Objects  0"));
}

#[test]
fn errors_exit_with_failure_and_a_message() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("p.copal");
    let f = f.to_str().unwrap();
    ok(&["new", f]);

    let out = cli(&["new", f]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("already exists"));

    let out = cli(&["run", f, "object.move", "id=42", "x=1"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no object with id 42"));

    let out = cli(&["info", "/definitely/missing.copal"]);
    assert!(!out.status.success());
    assert!(!Path::new("/definitely/missing.copal").exists());
}

#[test]
fn lists_commands() {
    let text = ok(&["commands"]);
    assert!(text.contains("object.add_cube"));
    let list: serde_json::Value = serde_json::from_str(&ok(&["--json", "commands"])).unwrap();
    assert!(list.as_array().unwrap().len() >= 10);
}

#[test]
fn settings_show_and_change() {
    let dir = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_copal-cli"))
            .env("COPAL_CONFIG_DIR", dir.path())
            .args(args)
            .output()
            .unwrap()
    };
    let out = run(&["settings"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("theme  = default"));

    let out = run(&["settings", "theme=art", "mode=light"]);
    assert!(out.status.success());
    let out = run(&["--json", "settings"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["theme"], "art");
    assert_eq!(v["mode"], "light");

    let out = run(&["settings", "theme=neon"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("default or art"));
}

fn cli_env(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_copal-cli"))
        .env("COPAL_CONFIG_DIR", dir.join("config"))
        .env("COPAL_DATA_DIR", dir.join("data"))
        .args(args)
        .output()
        .unwrap()
}

fn ok_env(dir: &Path, args: &[&str]) -> String {
    let out = cli_env(dir, args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn import_export_history_and_inspect() {
    let tmp = tempfile::tempdir().unwrap();
    let d = tmp.path();
    let stl = d.join("block.stl");
    // A 4 × 4 × 2 mm block as ASCII STL.
    let mut text = String::from("solid block\n");
    let v = |x: f32, y: f32, z: f32| format!("vertex {x} {y} {z}\n");
    let quads = [
        [(0., 0., 0.), (0., 4., 0.), (4., 4., 0.), (4., 0., 0.)],
        [(0., 0., 2.), (4., 0., 2.), (4., 4., 2.), (0., 4., 2.)],
        [(0., 0., 0.), (4., 0., 0.), (4., 0., 2.), (0., 0., 2.)],
        [(0., 4., 0.), (0., 4., 2.), (4., 4., 2.), (4., 4., 0.)],
        [(0., 0., 0.), (0., 0., 2.), (0., 4., 2.), (0., 4., 0.)],
        [(4., 0., 0.), (4., 4., 0.), (4., 4., 2.), (4., 0., 2.)],
    ];
    for q in quads {
        for t in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
            text.push_str("facet normal 0 0 0\nouter loop\n");
            for (x, y, z) in t {
                text.push_str(&v(x, y, z));
            }
            text.push_str("endloop\nendfacet\n");
        }
    }
    text.push_str("endsolid block\n");
    std::fs::write(&stl, text).unwrap();

    let project = d.join("job.copal");
    let p = project.to_str().unwrap();
    ok_env(
        d,
        &[
            "new",
            p,
            "--printer",
            "elegoo/saturn-4-ultra-16k",
            "--resin",
            "anycubic/abs-like-pro-2-grey",
        ],
    );
    ok_env(
        d,
        &[
            "import",
            p,
            stl.to_str().unwrap(),
            stl.to_str().unwrap(),
            "--arrange",
        ],
    );
    let info: serde_json::Value = serde_json::from_str(&ok_env(d, &["--json", "info", p])).unwrap();
    assert_eq!(info["objects"].as_array().unwrap().len(), 2);
    assert!(
        (info["settings"]["print"]["layer_height_mm"]
            .as_f64()
            .unwrap()
            - 0.03)
            .abs()
            < 1e-6
    );

    let goo = d.join("job.goo");
    let out = ok_env(d, &["export", p, "-o", goo.to_str().unwrap()]);
    assert!(out.contains("67 layers"), "{out}");
    let listed = ok_env(d, &["history"]);
    assert!(listed.contains("job.goo"), "{listed}");
    let list: serde_json::Value = serde_json::from_str(&ok_env(d, &["--json", "history"])).unwrap();
    let id = list[0]["id"].as_str().unwrap().to_string();
    ok_env(d, &["history", "rate", &id, "5", "good"]);
    assert!(ok_env(d, &["history", "show", &id]).contains("printed well, 5/5"));
    let copy = d.join("again.copal");
    ok_env(d, &["history", "open", &id, copy.to_str().unwrap()]);
    let again: serde_json::Value =
        serde_json::from_str(&ok_env(d, &["--json", "info", copy.to_str().unwrap()])).unwrap();
    assert_eq!(again["objects"].as_array().unwrap().len(), 2);

    let png = d.join("layer.png");
    let shown = ok_env(
        d,
        &[
            "inspect",
            goo.to_str().unwrap(),
            "--layer",
            "10",
            "--png",
            png.to_str().unwrap(),
        ],
    );
    assert!(shown.contains("All 67 layers decode correctly"), "{shown}");
    assert!(png.exists());
}

#[test]
fn presets_save_and_load() {
    let tmp = tempfile::tempdir().unwrap();
    let d = tmp.path();
    let a = d.join("a.copal");
    let b = d.join("b.copal");
    ok_env(
        d,
        &[
            "new",
            a.to_str().unwrap(),
            "--printer",
            "elegoo/saturn-4-ultra-16k",
        ],
    );
    ok_env(
        d,
        &[
            "new",
            b.to_str().unwrap(),
            "--printer",
            "elegoo/saturn-4-ultra-16k",
        ],
    );
    ok_env(
        d,
        &[
            "run",
            a.to_str().unwrap(),
            "print.set",
            "exposure_s=1.9",
            "bottom_layers=4",
        ],
    );
    ok_env(
        d,
        &[
            "presets",
            "save",
            a.to_str().unwrap(),
            "exposure",
            "My grey",
        ],
    );
    ok_env(
        d,
        &[
            "presets",
            "save",
            a.to_str().unwrap(),
            "resin",
            "Grey batch 24",
        ],
    );
    let list = ok_env(d, &["presets"]);
    assert!(
        list.contains("my-grey") && list.contains("grey-batch-24"),
        "{list}"
    );
    ok_env(
        d,
        &[
            "presets",
            "load",
            b.to_str().unwrap(),
            "exposure",
            "my-grey",
        ],
    );
    let info: serde_json::Value =
        serde_json::from_str(&ok_env(d, &["--json", "info", b.to_str().unwrap()])).unwrap();
    assert!((info["settings"]["print"]["exposure_s"].as_f64().unwrap() - 1.9).abs() < 1e-6);
    assert!(ok_env(d, &["resins"]).contains("my/grey-batch-24"));
    ok_env(
        d,
        &["presets", "load", b.to_str().unwrap(), "supports", "heavy"],
    );
    ok_env(d, &["presets", "delete", "exposure", "my-grey"]);
    assert!(!ok_env(d, &["presets"]).contains("my-grey"));
}
